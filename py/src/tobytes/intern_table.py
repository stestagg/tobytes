import msgpack
from typing import Optional, Callable, Any


INTERN_TABLE_EXT = 6


class Intern:
    """Wrapper to mark a value for interning.

    When encoding, values wrapped in Intern() will be added to the intern table
    and replaced with references.
    """
    def __init__(self, value: Any, by_identity: bool = True):
        """
        Args:
            value: The value to intern
            by_identity: If True, compare by identity (id()). If False, compare by equality.
        """
        self.value = value
        self.by_identity = by_identity


class InternPtr:
    """Represents a pointer/reference to an interned object."""
    def __init__(self, index: int):
        self.index = index

    def __repr__(self):
        return f"InternPtr({self.index})"


class InternTable:
    """Manages intern table for serialization/deserialization.

    During serialization:
    - Tracks objects to create intern entries
    - Returns references to already-interned objects

    During deserialization:
    - Stores interned objects for lookup
    - Validates that references only point to earlier entries (no forward references)
    """

    def __init__(self):
        self.table = []
        self.originals = []  # For equality comparison
        self.by_id = {}

    def __len__(self):
        return len(self.table)

    def __getitem__(self, index):
        if index < 0 or index >= len(self.table):
            raise IndexError(f"Intern table index {index} out of range (table size: {len(self.table)})")
        return self.table[index]

    def get_bytes(self):
        """Serialize the intern table as a msgpack array.

        Returns:
            Bytes containing msgpack array header followed by concatenated encoded entries
        """
        packer = msgpack.Packer()
        result = packer.pack_array_header(len(self.table))
        for entry_bytes in self.table:
            result += entry_bytes
        return result

    def load(self, data: bytes, unpacker_factory: Callable[[bytes], Any]):
        """Load intern table from bytes.

        Args:
            data: Raw bytes containing the msgpack array of interned objects
            unpacker_factory: Function to create unpacker (for handling custom ext types)
        """
        assert len(self.table) == 0, "Cannot load into non-empty intern table"

        unpacker = unpacker_factory(data)

        arr_len = unpacker.read_array_header()
        for _ in range(arr_len):
            entry = unpacker.unpack()
            self.table.append(entry)

    def put(self, obj: Any) -> tuple[int, bool]:
        """Add an object to the intern table if not already present.

        Args:
            obj: The object to intern

        Returns:
            tuple[int, bool]: (index, is_new) where index is the intern table
                              index and is_new indicates if this is a new entry
        """
        key = id(obj)
        if key in self.by_id:
            return self.by_id[key], False

        idx = len(self.table)
        self.table.append(obj)
        self.by_id[key] = idx

        return idx, True

    def _find(self, value: Any, by_identity: bool) -> Optional[int]:
        """Find a value in the intern table.

        Args:
            value: The value to find
            by_identity: If True, compare by identity (id()). If False, compare by equality.

        Returns:
            The index if found, None otherwise
        """
        if by_identity:
            key = id(value)
            return self.by_id.get(key)
        else:
            for idx, original in enumerate(self.originals):
                if original == value:
                    return idx
            return None

    def intern(self, intern_wrapper: 'Intern', encoder_callback: Callable[[Any], bytes]) -> msgpack.ExtType:
        """Intern a value and return a reference to it.

        Args:
            intern_wrapper: The Intern wrapper containing the value
            encoder_callback: Function to encode the value to bytes

        Returns:
            ExtType representing the intern reference
        """
        value = intern_wrapper.value
        by_identity = intern_wrapper.by_identity

        existing_idx = self._find(value, by_identity)
        if existing_idx is not None:
            return self.create_reference(existing_idx)

        # Encode the value first - this handles nested Interns and ensures topological order
        encoded_bytes = encoder_callback(value)

        idx = len(self.table)
        self.table.append(encoded_bytes)
        self.originals.append(value)

        if by_identity:
            self.by_id[id(value)] = idx

        return self.create_reference(idx)

    def create_reference(self, index: int) -> msgpack.ExtType:
        """Create an intern reference pointing to the given index.

        Args:
            index: Index in the intern table

        Returns:
            msgpack.ExtType representing the reference
        """
        return msgpack.ExtType(INTERN_TABLE_EXT, msgpack.packb(index))


class InternContext:
    """Context for handling intern table during serialization/deserialization."""

    def __init__(self):
        self.table: Optional[InternTable] = None
        self.active = False

    def start_table(self):
        """Start a new intern table context."""
        if self.active:
            raise ValueError("Cannot nest intern tables")
        self.table = InternTable()
        self.active = True

    def end_table(self):
        """End the current intern table context."""
        table = self.table
        self.table = None
        self.active = False
        return table

    def is_active(self) -> bool:
        """Check if we're currently within an intern table context."""
        return self.active

    def get_table(self) -> Optional[InternTable]:
        """Get the current intern table, if any."""
        return self.table

    def ensure_table(self) -> InternTable:
        """Ensure an intern table exists, creating one if needed.

        Returns:
            The intern table
        """
        if not self.active:
            self.start_table()
        return self.table

    def intern(self, intern_wrapper: Intern, encoder_callback: Callable[[Any], bytes]) -> msgpack.ExtType:
        """Intern a value and return a reference to it.

        Args:
            intern_wrapper: The Intern wrapper containing the value
            encoder_callback: Function to encode the value to bytes

        Returns:
            ExtType representing the intern reference
        """
        table = self.ensure_table()
        return table.intern(intern_wrapper, encoder_callback)

    def maybe_wrap_with_table(self, data_bytes: bytes) -> bytes:
        """Wrap data with intern table if one was created, otherwise return data as-is.

        Args:
            data_bytes: The encoded data bytes

        Returns:
            Either the intern table wrapped data, or the original data bytes
        """
        try:
            if self.active and self.table and len(self.table) > 0:
                payload = self.table.get_bytes() + data_bytes
                intern_ext = msgpack.ExtType(INTERN_TABLE_EXT, payload)
                return msgpack.packb(intern_ext)
            else:
                return data_bytes
        finally:
            if self.active:
                self.end_table()

    def handle_intern_reference(self, data: bytes) -> Any:
        """Handle an intern reference (Ext 6 within an active intern table).

        Args:
            data: Raw bytes containing the reference index

        Returns:
            The interned object at the specified index

        Raises:
            ValueError: If there's no active intern table or forward reference detected
            IndexError: If the reference index is out of bounds
        """
        ref_index = msgpack.unpackb(data, raw=False)

        if self.table is None:
            raise ValueError("Intern reference found but no active intern table")

        # Per spec: references must point to earlier entries (lower indices)
        if ref_index >= len(self.table):
            raise ValueError(
                f"Forward reference detected: index {ref_index} references "
                f"entry not yet loaded (table size: {len(self.table)}). "
                f"Intern table entries must only reference earlier entries."
            )

        return self.table[ref_index]

    def decode_intern_table(self, data: bytes, ext_hook: Callable[[int, bytes], Any]) -> Any:
        """Decode an intern table structure (Ext 6 outside an active intern table).

        Args:
            data: Raw bytes containing the intern table structure
            ext_hook: Extension hook function to use for nested decoding

        Returns:
            The decoded data with intern references resolved
        """
        unpacker = msgpack.Unpacker(raw=False)
        unpacker.feed(data)

        start_pos = 0
        _ = unpacker.unpack()
        end_pos = unpacker.tell()

        interned_objects_bytes = data[start_pos:end_pos]
        data_bytes = data[end_pos:]

        self.start_table()

        try:
            def make_unpacker(unpacker_data: bytes):
                """Create an unpacker with the ext_hook configured."""
                up = msgpack.Unpacker(raw=False, ext_hook=ext_hook)
                up.feed(unpacker_data)
                return up

            self.table.load(interned_objects_bytes, make_unpacker)

            result = msgpack.unpackb(data_bytes, raw=False, ext_hook=ext_hook)

            return result
        finally:
            self.end_table()

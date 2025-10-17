import msgpack
from collections.abc import Callable
from dataclasses import dataclass
from typing import Optional

from .intern_table import (
    InternTable,
    InternContext,
    InternPtr,
    Intern,
    INTERN_TABLE_EXT
)


CUSTOM_TYPE_EXT = 8


@dataclass
class EncodedCustomType:
    namespace: str
    type_id: int
    data: bytes


@dataclass
class CustomTypeCodec:
    py_type: type
    encoder: Callable[['Codec', object], bytes]
    decoder: Callable[['Codec', bytes], object]
    match_subtypes: bool = False
    
    def matches(self, obj: object) -> bool:
        if self.match_subtypes:
            return isinstance(obj, self.py_type)
        else:
            return type(obj) is self.py_type
        

class CustomNameSpace:

    def matches(self, obj: object) -> bool:
        raise NotImplementedError(f'{type(self)} does not implement matches')
    
    def encode(self, codec: 'Codec', obj: object) -> bytes:
        raise NotImplementedError(f'{type(self)} does not implement encode')

    def decode(self, codec: 'Codec', data: bytes) -> object:
        raise NotImplementedError(f'{type(self)} does not implement decode')


NameSpace = dict[int, CustomTypeCodec] | CustomNameSpace
NameSpaces = dict[str, NameSpace]


class Codec:

    def __init__(self, namespaces: Optional[NameSpaces]=None):
        self.namespaces = dict(namespaces) if namespaces else {}
        self._intern_context = InternContext()

    def add_namespace(self, namespace: str, types: NameSpace):
        if namespace in self.namespaces:
            raise ValueError(f"Namespace '{namespace}' already exists.")
        self.namespaces[namespace] = types

    def _encode_custom_type(self, namespace: str, type_id: int, codec: CustomTypeCodec, obj) -> msgpack.ExtType:
        encoded_data = codec.encoder(self, obj)
        namespace_bytes = msgpack.packb(namespace)
        type_id_bytes = msgpack.packb(type_id)
        payload = namespace_bytes + type_id_bytes + encoded_data
        return msgpack.ExtType(CUSTOM_TYPE_EXT, payload)

    def _default_encoder(self, obj):
        if isinstance(obj, Intern):
            encoder = lambda val: msgpack.packb(val, default=self._default_encoder, strict_types=True)
            return self._intern_context.intern(obj, encoder)

        for namespace, types in self.namespaces.items():
            if isinstance(types, CustomNameSpace):
                if types.matches(obj):
                    encoded_data = types.encode(self, obj)
                    namespace_bytes = msgpack.packb(namespace)
                    type_id_bytes = msgpack.packb(0)
                    payload = namespace_bytes + type_id_bytes + encoded_data
                    return msgpack.ExtType(CUSTOM_TYPE_EXT, payload)
            else:
                for type_id, codec in types.items():
                    if codec.matches(obj):
                        return self._encode_custom_type(namespace, type_id, codec, obj)
        raise TypeError(f"Cannot serialize object of type {type(obj)}")

    def _ext_hook(self, code, data):
        if code == INTERN_TABLE_EXT:
            if self._intern_context.is_active():
                return self._intern_context.handle_intern_reference(data)
            else:
                return self._intern_context.decode_intern_table(data, self._ext_hook)

        if code == CUSTOM_TYPE_EXT:
            unpacker = msgpack.Unpacker(raw=False)
            unpacker.feed(data)

            namespace = next(unpacker)
            type_id = next(unpacker)
            remaining_data = data[unpacker.tell():]

            if namespace not in self.namespaces:
                raise ValueError(f"Unknown namespace: {namespace}")

            types = self.namespaces[namespace]

            if isinstance(types, CustomNameSpace):
                return types.decode(self, remaining_data)
            else:
                if type_id not in types:
                    raise ValueError(f"Unknown type_id {type_id} in namespace {namespace}")

                codec = types[type_id]
                return codec.decoder(self, remaining_data)

        return msgpack.ExtType(code, data)

    def dumps(self, obj) -> bytes:
        """Serialize an object to bytes.

        If the object contains Intern() wrappers, an intern table will be created automatically.

        Args:
            obj: The object to serialize

        Returns:
            Serialized bytes
        """
        if self._intern_context.active:
            self._intern_context.end_table()

        data_bytes = msgpack.packb(obj, default=self._default_encoder, strict_types=True)

        return self._intern_context.maybe_wrap_with_table(data_bytes)

    def loads(self, data: bytes):
        return msgpack.unpackb(data, ext_hook=self._ext_hook, raw=False)
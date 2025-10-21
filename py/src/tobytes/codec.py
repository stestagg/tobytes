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
    
    def matches(self, obj: object) -> bool:
        return type(obj) is self.py_type
        

class CustomNameSpace:

    def matches(self, obj: object) -> bool:
        raise NotImplementedError(f'{type(self)} does not implement matches')
    
    def encode(self, codec: 'Codec', obj: object) -> bytes:
        raise NotImplementedError(f'{type(self)} does not implement encode')

    def decode(self, codec: 'Codec', data: bytes) -> object:
        raise NotImplementedError(f'{type(self)} does not implement decode')
    

class NamespaceModule:

    @dataclass
    class TypeCodec:
        py_type: type
        type_id: int
        encoder: Optional[Callable[['Codec', object], bytes]] = None
        decoder: Optional[Callable[['Codec', bytes], object]] = None

    def __init__(self, name: str):
        self.name = name
        self.codecs = []

    def _check_unique_id(self, type_id: int):
        for codec in self.codecs:
            if codec.type_id == type_id:
                raise ValueError(f"Type ID {type_id} is already used in namespace '{self.name}'")

    def encoder(self, py_type: type, type_id: int):
        self._check_unique_id(type_id)
        codec = self.TypeCodec(
            py_type=py_type,
            type_id=type_id,
        )
        self.codecs.append(codec)

        def decorate_decode_fn(decode_fn: Callable[['Codec', object], bytes]):
            codec.decoder = decode_fn
            return decode_fn
        
        def decorate_encode_fn(encode_fn: Callable[['Codec', bytes], object]):
            codec.encoder = encode_fn # type: ignore
            encode_fn.decoder = decorate_decode_fn
            return encode_fn
        return decorate_encode_fn
    
    def decoder(self, py_type: type, type_id: int):
        self._check_unique_id(type_id)
        codec = self.TypeCodec(
            py_type=py_type,
            type_id=type_id,
        )
        self.codecs.append(codec)

        def decorate_encode_fn(encode_fn: Callable[['Codec', object], bytes]):
            codec.encoder = encode_fn
            return encode_fn
        
        def decorate_decode_fn(decode_fn: Callable[['Codec', bytes], object]):
            codec.decoder = decode_fn
            decode_fn.encoder = decorate_encode_fn
            return decode_fn
        return decorate_decode_fn
    
    def custom_types(self) -> dict[int, CustomTypeCodec]:
        result = {}
        for codec in self.codecs:
            result[codec.type_id] = CustomTypeCodec(
                py_type=codec.py_type,
                encoder=codec.encoder,
                decoder=codec.decoder,
            )
        return result

Namespace = dict[int, CustomTypeCodec] | CustomNameSpace
Namespaces = dict[str, Namespace]


class Codec:

    def __init__(self, namespaces: Optional[Namespaces]=None):
        self.namespaces = {}
        self._intern_context = InternContext()
        self._type_map = {}
        self._add_default_namespaces()
        if namespaces:
            self.namespaces.update(namespaces)
        self._rebuild_type_map()

    def _add_default_namespaces(self):
        """Add built-in default namespaces to the codec."""
        from .table import table_namespace
        self.namespaces[table_namespace.name] = table_namespace.custom_types()

    def clear_namespaces(self):
        """Remove all namespaces from the codec, including default namespaces."""
        self.namespaces.clear()
        self._rebuild_type_map()

    def _rebuild_type_map(self):
        self._type_map = {}
        for namespace, types in self.namespaces.items():
            if isinstance(types, CustomNameSpace):
                continue
            for type_id, codec in types.items():
                self._type_map[codec.py_type] = (namespace, type_id, codec)

    def add_namespace(self, namespace: str, types: Namespace):
        if namespace in self.namespaces:
            raise ValueError(f"Namespace '{namespace}' already exists.")
        self.namespaces[namespace] = types
        self._rebuild_type_map()

    def add_module(self, module: NamespaceModule):
        if module.name in self.namespaces and self.namespaces[module.name] is not module:
            raise ValueError(f"Namespace '{module.name}' already exists.")
        self.namespaces[module.name] = module.custom_types()
        self._rebuild_type_map()

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

        py_type = type(obj)
        if py_type in self._type_map:
            namespace, type_id, codec = self._type_map[py_type]
            return self._encode_custom_type(namespace, type_id, codec, obj)
        
        for namespace, types in self.namespaces.items():
            if isinstance(types, CustomNameSpace):
                if types.matches(obj):
                    encoded_data = types.encode(self, obj)
                    namespace_bytes = msgpack.packb(namespace)
                    type_id_bytes = msgpack.packb(0)
                    payload = namespace_bytes + type_id_bytes + encoded_data
                    return msgpack.ExtType(CUSTOM_TYPE_EXT, payload)
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
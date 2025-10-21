import tobytes
import pytest

class Bob:
    def __init__(self, name: str):
        self.name = name
    

class Bill(Bob):
    pass


def test_custom_namespace():

    mod = tobytes.NamespaceModule("test_namespace")

    @mod.encoder(py_type=Bob, type_id=1)
    def encode_bob(codec: tobytes.Codec, obj: Bob) -> bytes:
        return codec.dumps(obj.name)
    
    @encode_bob.decoder
    def decode_bob(codec: tobytes.Codec, data: bytes) -> Bob:
        name = codec.loads(data)
        return Bob(name)
    
    codec = tobytes.Codec()

    with pytest.raises(TypeError):
        codec.dumps(Bob("Alice"))

    codec.add_module(mod)

    encoded = codec.dumps(Bob("Alice"))
    decoded = codec.loads(encoded)
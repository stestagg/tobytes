import tobytes

def test_simple():
    codec = tobytes.Codec()

    assert codec.dumps(1) == b'\x01'

class MyType:
    def __init__(self, value):
        self.value = value

def test_custom():
    codec = tobytes.Codec()

    codec.add_namespace("tobytes.test", {
        1: tobytes.CustomTypeCodec(
            py_type=MyType,
            encoder=lambda enc, obj: enc.dumps(obj.value),
            decoder=lambda dec, data: MyType(dec.loads(data))
        )
    })

    obj = MyType(1)
    serialized = codec.dumps(obj)
    deserialized = codec.loads(serialized)
    assert isinstance(deserialized, MyType)
    assert deserialized.value == 1

class AnotherType:
    def __init__(self, value):
        self.value = value

class MyCustomNameSpace(tobytes.CustomNameSpace):
    def matches(self, obj: object) -> bool:
        return isinstance(obj, AnotherType)

    def encode(self, codec: tobytes.Codec, obj: object) -> bytes:
        return codec.dumps(obj.value)

    def decode(self, codec: tobytes.Codec, data: bytes) -> object:
        return AnotherType(codec.loads(data))

def test_custom_namespace():
    codec = tobytes.Codec()

    codec.add_namespace("tobytes.test.custom", MyCustomNameSpace())

    obj = AnotherType(42)
    serialized = codec.dumps(obj)
    deserialized = codec.loads(serialized)
    assert isinstance(deserialized, AnotherType)
    assert deserialized.value == 42
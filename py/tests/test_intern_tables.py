"""
Tests for intern table functionality (Extension 0x06)

Intern tables allow reducing message size by referencing repeated objects.
Format: [Ext 6, array interned_objects, any data]
Intern references: [Ext 6, uint] where uint is the intern ID
"""
import msgpack
import pytest
import tobytes


def test_intern_table_basic_structure():
    """Test basic intern table encoding and decoding"""
    codec = tobytes.Codec()

    # Manually construct an intern table message:
    # Ext 6 containing:
    #   - interned_objects array: ["hello", "world"]
    #   - data: [ref(0), ref(1), ref(0)]  # "hello", "world", "hello"

    interned_objects = ["hello", "world"]

    ref_0 = msgpack.ExtType(6, msgpack.packb(0))
    ref_1 = msgpack.ExtType(6, msgpack.packb(1))

    data = [ref_0, ref_1, ref_0]

    payload = msgpack.packb(interned_objects) + msgpack.packb(data)
    intern_table = msgpack.ExtType(6, payload)

    serialized = msgpack.packb(intern_table)

    result = codec.loads(serialized)
    assert result == ["hello", "world", "hello"]


def test_intern_table_with_repeated_objects():
    """Test that intern tables correctly deduplicate repeated objects"""
    codec = tobytes.Codec()

    repeated_str = "repeated_value"
    interned_objects = [repeated_str, 123]

    ref_0 = msgpack.ExtType(6, msgpack.packb(0))
    ref_1 = msgpack.ExtType(6, msgpack.packb(1))

    data = {
        "a": ref_0,
        "b": ref_1,
        "c": ref_0,
        "d": ref_0,
    }

    payload = msgpack.packb(interned_objects) + msgpack.packb(data)
    intern_table = msgpack.ExtType(6, payload)
    serialized = msgpack.packb(intern_table)

    result = codec.loads(serialized)
    assert result["a"] == "repeated_value"
    assert result["b"] == 123
    assert result["c"] == "repeated_value"
    assert result["d"] == "repeated_value"


def test_intern_table_with_forward_references():
    """Test that forward references (references to later entries) cause an error"""
    codec = tobytes.Codec()

    # Create an intern table where earlier entries reference later entries
    # interned_objects[0] = [ref(1), ref(2)]  # Forward references - NOT ALLOWED
    # interned_objects[1] = "hello"
    # interned_objects[2] = "world"

    ref_1 = msgpack.ExtType(6, msgpack.packb(1))
    ref_2 = msgpack.ExtType(6, msgpack.packb(2))

    interned_objects = [
        [ref_1, ref_2],
        "hello",
        "world"
    ]

    ref_0 = msgpack.ExtType(6, msgpack.packb(0))
    data = {"result": ref_0}

    payload = msgpack.packb(interned_objects) + msgpack.packb(data)
    intern_table = msgpack.ExtType(6, payload)
    serialized = msgpack.packb(intern_table)

    with pytest.raises(ValueError, match="Forward reference detected"):
        codec.loads(serialized)


def test_intern_table_with_nested_structures():
    """Test intern tables with nested data structures"""
    codec = tobytes.Codec()

    interned_objects = [
        {"name": "Alice", "age": 30},
        {"name": "Bob", "age": 25},
        ["tag1", "tag2", "tag3"]
    ]

    ref_0 = msgpack.ExtType(6, msgpack.packb(0))
    ref_1 = msgpack.ExtType(6, msgpack.packb(1))
    ref_2 = msgpack.ExtType(6, msgpack.packb(2))

    data = {
        "users": [ref_0, ref_1, ref_0],
        "tags": ref_2,
        "featured": ref_1
    }

    payload = msgpack.packb(interned_objects) + msgpack.packb(data)
    intern_table = msgpack.ExtType(6, payload)
    serialized = msgpack.packb(intern_table)

    result = codec.loads(serialized)
    assert result["users"][0] == {"name": "Alice", "age": 30}
    assert result["users"][1] == {"name": "Bob", "age": 25}
    assert result["users"][2] == {"name": "Alice", "age": 30}
    assert result["tags"] == ["tag1", "tag2", "tag3"]
    assert result["featured"] == {"name": "Bob", "age": 25}


def test_intern_reference_without_table():
    """Test that intern references without a surrounding table cause an error"""
    codec = tobytes.Codec()

    ref = msgpack.ExtType(6, msgpack.packb(0))
    serialized = msgpack.packb(ref)

    with pytest.raises(Exception):
        codec.loads(serialized)


def test_intern_table_invalid_reference():
    """Test that invalid intern references (out of bounds) cause an error"""
    codec = tobytes.Codec()

    interned_objects = ["hello", "world"]

    ref_invalid = msgpack.ExtType(6, msgpack.packb(10))
    data = ref_invalid

    payload = msgpack.packb(interned_objects) + msgpack.packb(data)
    intern_table = msgpack.ExtType(6, payload)
    serialized = msgpack.packb(intern_table)

    with pytest.raises(Exception):
        codec.loads(serialized)


def test_intern_table_no_nested_intern_tables():
    """Test that intern tables cannot be nested within each other"""
    codec = tobytes.Codec()

    # According to the spec, intern tables cannot contain other intern tables
    # because Ext 6 is repurposed for references within the data section

    inner_interned = ["inner"]
    inner_ref = msgpack.ExtType(6, msgpack.packb(0))
    inner_payload = msgpack.packb(inner_interned) + msgpack.packb(inner_ref)
    inner_table = msgpack.ExtType(6, inner_payload)

    outer_interned = [inner_table, "outer"]
    outer_ref_0 = msgpack.ExtType(6, msgpack.packb(0))
    outer_payload = msgpack.packb(outer_interned) + msgpack.packb(outer_ref_0)
    outer_table = msgpack.ExtType(6, outer_payload)

    serialized = msgpack.packb(outer_table)

    with pytest.raises(Exception):
        codec.loads(serialized)


def test_intern_table_empty():
    """Test intern table with empty interned_objects array"""
    codec = tobytes.Codec()

    interned_objects = []
    data = "just a string"

    payload = msgpack.packb(interned_objects) + msgpack.packb(data)
    intern_table = msgpack.ExtType(6, payload)
    serialized = msgpack.packb(intern_table)

    result = codec.loads(serialized)
    assert result == "just a string"


def test_intern_table_with_none_values():
    """Test intern table with None/null values"""
    codec = tobytes.Codec()

    interned_objects = [None, "value", None]

    ref_0 = msgpack.ExtType(6, msgpack.packb(0))
    ref_1 = msgpack.ExtType(6, msgpack.packb(1))
    ref_2 = msgpack.ExtType(6, msgpack.packb(2))

    data = [ref_0, ref_1, ref_2]

    payload = msgpack.packb(interned_objects) + msgpack.packb(data)
    intern_table = msgpack.ExtType(6, payload)
    serialized = msgpack.packb(intern_table)

    result = codec.loads(serialized)
    assert result == [None, "value", None]


def test_intern_table_chain_references():
    """Test intern table where references point to objects containing other references (backward refs only)"""
    codec = tobytes.Codec()

    # Create a chain with backward references only:
    # interned_objects[0] = "final value"
    # interned_objects[1] = ["second", ref(0)]
    # interned_objects[2] = ["first", ref(1)]

    ref_0 = msgpack.ExtType(6, msgpack.packb(0))
    ref_1 = msgpack.ExtType(6, msgpack.packb(1))

    interned_objects = [
        "final value",
        ["second", ref_0],
        ["first", ref_1]
    ]

    ref_2 = msgpack.ExtType(6, msgpack.packb(2))
    data = ref_2

    payload = msgpack.packb(interned_objects) + msgpack.packb(data)
    intern_table = msgpack.ExtType(6, payload)
    serialized = msgpack.packb(intern_table)

    result = codec.loads(serialized)
    assert result == ["first", ["second", "final value"]]


# Tests for serialization (encoding with intern tables)

def test_serialize_with_intern_table():
    """Test that codec can serialize objects using intern tables with Intern() wrapper"""
    codec = tobytes.Codec()

    repeated = "repeated_string"
    data = {
        "a": tobytes.Intern(repeated),
        "b": tobytes.Intern(repeated),
        "c": tobytes.Intern(repeated),
        "d": [tobytes.Intern(repeated), tobytes.Intern(repeated)]
    }

    serialized = codec.dumps(data)

    result = codec.loads(serialized)
    expected = {
        "a": repeated,
        "b": repeated,
        "c": repeated,
        "d": [repeated, repeated]
    }
    assert result == expected


def test_serialize_without_intern_table():
    """Test that serialization works without intern tables when no Intern() wrappers are used"""
    codec = tobytes.Codec()

    data = {
        "a": "value1",
        "b": "value2",
        "c": "value1",
    }

    serialized = codec.dumps(data)
    result = codec.loads(serialized)
    assert result == data

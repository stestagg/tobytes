import tobytes
import pytest

try:
    import numpy as np
    HAS_NUMPY = True
except ImportError:
    HAS_NUMPY = False


@pytest.mark.skipif(not HAS_NUMPY, reason="numpy not installed")
def test_numpy_array_default_namespace():
    """Test that numpy arrays are serialized with the default table namespace."""
    codec = tobytes.Codec()

    # Create a simple numpy array
    arr = np.array([1, 2, 3, 4, 5])

    # Encode and decode
    encoded = codec.dumps(arr)
    decoded = codec.loads(encoded)

    # Verify the result
    assert isinstance(decoded, np.ndarray)
    assert np.array_equal(decoded, arr)


@pytest.mark.skipif(not HAS_NUMPY, reason="numpy not installed")
def test_numpy_multidimensional_array():
    """Test serialization of multidimensional numpy arrays."""
    codec = tobytes.Codec()

    # Create a 2D array
    arr = np.array([[1, 2, 3], [4, 5, 6]])

    encoded = codec.dumps(arr)
    decoded = codec.loads(encoded)

    assert isinstance(decoded, np.ndarray)
    assert np.array_equal(decoded, arr)
    assert decoded.shape == arr.shape


@pytest.mark.skipif(not HAS_NUMPY, reason="numpy not installed")
def test_numpy_array_dtypes():
    """Test serialization of numpy arrays with different dtypes."""
    codec = tobytes.Codec()

    # Test different data types
    dtypes = [np.int32, np.float64, np.complex128, np.bool_]

    for dtype in dtypes:
        arr = np.array([1, 2, 3], dtype=dtype)
        encoded = codec.dumps(arr)
        decoded = codec.loads(encoded)

        assert isinstance(decoded, np.ndarray)
        assert np.array_equal(decoded, arr)
        assert decoded.dtype == arr.dtype


@pytest.mark.skipif(not HAS_NUMPY, reason="numpy not installed")
def test_clear_namespaces():
    """Test that clear_namespaces removes the table namespace."""
    codec = tobytes.Codec()

    # Should work with default namespace
    arr = np.array([1, 2, 3])
    encoded = codec.dumps(arr)
    decoded = codec.loads(encoded)
    assert np.array_equal(decoded, arr)

    # Clear namespaces
    codec.clear_namespaces()

    # Should now fail to encode
    with pytest.raises(TypeError):
        codec.dumps(arr)


@pytest.mark.skipif(not HAS_NUMPY, reason="numpy not installed")
def test_table_namespace_readdable_after_clear():
    """Test that we can re-add the table namespace after clearing."""
    from tobytes.table import table_namespace

    codec = tobytes.Codec()
    codec.clear_namespaces()

    # Should fail without namespace
    arr = np.array([1, 2, 3])
    with pytest.raises(TypeError):
        codec.dumps(arr)

    # Re-add the table namespace
    codec.add_module(table_namespace)

    # Should work again
    encoded = codec.dumps(arr)
    decoded = codec.loads(encoded)
    assert np.array_equal(decoded, arr)


@pytest.mark.skipif(not HAS_NUMPY, reason="numpy not installed")
def test_numpy_array_in_complex_structure():
    """Test that numpy arrays work within complex nested structures."""
    codec = tobytes.Codec()

    data = {
        "arrays": [
            np.array([1, 2, 3]),
            np.array([[4, 5], [6, 7]]),
        ],
        "metadata": {
            "name": "test",
            "value": np.array([1.5, 2.5, 3.5]),
        }
    }

    encoded = codec.dumps(data)
    decoded = codec.loads(encoded)

    assert isinstance(decoded, dict)
    assert len(decoded["arrays"]) == 2
    assert np.array_equal(decoded["arrays"][0], data["arrays"][0])
    assert np.array_equal(decoded["arrays"][1], data["arrays"][1])
    assert np.array_equal(decoded["metadata"]["value"], data["metadata"]["value"])

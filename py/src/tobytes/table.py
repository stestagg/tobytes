"""Table namespace for tobytes codec.

Provides serialization support for tabular types including numpy arrays.
"""
import io
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from .codec import Codec

try:
    import numpy as np
    HAS_NUMPY = True
except ImportError:
    HAS_NUMPY = False

from .codec import NamespaceModule


table_namespace = NamespaceModule("table")


if HAS_NUMPY:
    @table_namespace.encoder(py_type=np.ndarray, type_id=1)
    def encode_ndarray(codec: 'Codec', obj: np.ndarray) -> bytes:
        """Encode numpy array using numpy's native format."""
        buf = io.BytesIO()
        np.save(buf, obj, allow_pickle=False)
        return buf.getvalue()

    @encode_ndarray.decoder
    def decode_ndarray(codec: 'Codec', data: bytes) -> np.ndarray:
        """Decode numpy array from numpy's native format."""
        buf = io.BytesIO(data)
        return np.load(buf, allow_pickle=False)


__all__ = ['table_namespace']

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

try:
    import pandas as pd
    HAS_PANDAS = True
except ImportError:
    HAS_PANDAS = False

try:
    import polars as pl
    HAS_POLARS = True
except ImportError:
    HAS_POLARS = False

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


if HAS_PANDAS:
    @table_namespace.encoder(py_type=pd.DataFrame, type_id=2)
    def encode_pandas_dataframe(codec: 'Codec', obj: 'pd.DataFrame') -> bytes:
        """Encode pandas DataFrame to parquet format."""
        buf = io.BytesIO()
        obj.to_parquet(buf, index=True)
        return buf.getvalue()

    @encode_pandas_dataframe.decoder
    def decode_pandas_dataframe(codec: 'Codec', data: bytes) -> 'pd.DataFrame':
        """Decode pandas DataFrame from parquet bytes."""
        buf = io.BytesIO(data)
        buf.seek(0)
        return pd.read_parquet(buf)


if HAS_POLARS:
    @table_namespace.encoder(py_type=pl.DataFrame, type_id=3)
    def encode_polars_dataframe(codec: 'Codec', obj: 'pl.DataFrame') -> bytes:
        """Encode polars DataFrame to parquet format."""
        buf = io.BytesIO()
        obj.write_parquet(buf)
        return buf.getvalue()

    @encode_polars_dataframe.decoder
    def decode_polars_dataframe(codec: 'Codec', data: bytes) -> 'pl.DataFrame':
        """Decode polars DataFrame from parquet bytes."""
        buf = io.BytesIO(data)
        buf.seek(0)
        return pl.read_parquet(buf)


__all__ = ['table_namespace']

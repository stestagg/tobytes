use crate::decode::read_ns_payload;
use crate::{Namespace, NamespaceEncodedValue, ToBytesResult};
use ndarray::{Data, Dimension};
use ndarray_npy::{ReadNpyExt, WriteNpyExt};
use std::io::Read;

#[cfg(feature = "polars")]
use polars::io::parquet::{ParquetReader, ParquetWriter};
#[cfg(feature = "polars")]
use polars::prelude::{DataFrame as PolarsDataFrame, SerReader};

pub trait ToTableNs {
    fn to_table_ns(&self) -> ToBytesResult<NamespaceEncodedValue>;
}

pub trait FromTableNs: Sized {
    fn from_table_ns<R: Read>(rd: &mut R) -> ToBytesResult<Self>;
}

impl<S, D> ToTableNs for ndarray::ArrayBase<S, D>
where
    S: Data,
    D: Dimension,
    ndarray::ArrayBase<S, D>: WriteNpyExt,
{
    fn to_table_ns(&self) -> ToBytesResult<NamespaceEncodedValue> {
        let buf = Vec::new();
        let mut wr = std::io::Cursor::new(buf);
        self.write_npy(&mut wr)?;
        Ok(NamespaceEncodedValue {
            namespace: "table",
            id: 1,
            value: wr.into_inner(),
        })
    }
}

impl<S, D> FromTableNs for ndarray::ArrayBase<S, D>
where
    S: Data,
    D: Dimension,
    ndarray::ArrayBase<S, D>: ReadNpyExt,
{
    fn from_table_ns<R: std::io::Read>(rd: &mut R) -> ToBytesResult<Self> {
        let payload = read_ns_payload(rd, "table", 1)?;
        Ok(Self::read_npy(&mut std::io::Cursor::new(payload))?)
    }
}

#[cfg(feature = "polars")]
impl ToTableNs for PolarsDataFrame {
    fn to_table_ns(&self) -> ToBytesResult<NamespaceEncodedValue> {
        let mut buffer = Vec::new();
        {
            let mut cursor = std::io::Cursor::new(&mut buffer);
            let mut df_clone = self.clone();
            ParquetWriter::new(&mut cursor).finish(&mut df_clone)?;
        }

        Ok(NamespaceEncodedValue {
            namespace: "table",
            id: 3,
            value: buffer,
        })
    }
}

#[cfg(feature = "polars")]
impl FromTableNs for PolarsDataFrame {
    fn from_table_ns<R: std::io::Read>(rd: &mut R) -> ToBytesResult<Self> {
        let payload = read_ns_payload(rd, "table", 3)?;
        let cursor = std::io::Cursor::new(payload);
        Ok(ParquetReader::new(cursor).finish()?)
    }
}

struct TableNamespace;

impl Namespace for TableNamespace {
    fn name() -> &'static str {
        "table"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    use crate::encode::ToBytes;

    #[cfg(feature = "polars")]
    use polars::prelude::{DataFrame as TestDataFrame, Series};

    #[rstest]
    fn test_table_namespace_encoding() {
        let value = ndarray::array![[1u8, 2u8], [3u8, 4u8]];
        let ns_value = value.to_table_ns().unwrap();
        assert_eq!(ns_value.namespace, "table");
        assert_eq!(ns_value.id, 1);
        let expected_bytes = vec![
            147, 78, 85, 77, 80, 89, 1, 0, 118, 0, 123, 39, 100, 101, 115, 99, 114, 39, 58, 32, 39,
            124, 117, 49, 39, 44, 32, 39, 102, 111, 114, 116, 114, 97, 110, 95, 111, 114, 100, 101,
            114, 39, 58, 32, 70, 97, 108, 115, 101, 44, 32, 39, 115, 104, 97, 112, 101, 39, 58, 32,
            40, 50, 44, 32, 50, 41, 125, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
            32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
            32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
            32, 32, 10, 1, 2, 3, 4,
        ];
        assert_eq!(ns_value.value, expected_bytes);

        let buf: &mut Vec<u8> = &mut Vec::new();
        ns_value.to_bytes(buf).unwrap();

        let mut expected = vec![
            0xc7,       // ext 8
            139,        // 139 byte payload
            8,          // custom type ext
            0b10100101, // b10100000 & 5 - 'table' str of length 5
            116, 97, 98, 108, 101, // 'table'
            1,   // numpy type id
        ];
        expected.extend_from_slice(&expected_bytes);
        assert_eq!(buf.as_slice(), expected.as_slice());
    }

    #[rstest]
    fn test_table_round_trip() {
        let value = ndarray::array![[10u8, 20u8], [30u8, 40u8]];
        let ns_value = value.to_table_ns().unwrap();

        let mut buf: &mut Vec<u8> = &mut Vec::new();
        ns_value.to_bytes(buf).unwrap();

        let decoded_value: ndarray::Array2<u8> =
            FromTableNs::from_table_ns(&mut std::io::Cursor::new(buf)).unwrap();
        assert_eq!(value, decoded_value);
    }

    #[cfg(feature = "polars")]
    #[rstest]
    fn test_polars_table_round_trip() {
        let df = TestDataFrame::new(vec![
            Series::new("id", &[1i64, 2, 3]),
            Series::new("value", &["a", "b", "c"]),
        ])
        .unwrap();

        let ns_value = df.to_table_ns().unwrap();
        assert_eq!(ns_value.namespace, "table");
        assert_eq!(ns_value.id, 3);

        let mut buf: &mut Vec<u8> = &mut Vec::new();
        ns_value.to_bytes(buf).unwrap();

        let decoded: TestDataFrame =
            FromTableNs::from_table_ns(&mut std::io::Cursor::new(buf)).unwrap();
        assert!(decoded.frame_equal(&df));
    }
}

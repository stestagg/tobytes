use crate::ToBytesResult;
use rmpv::decode::read_value;

pub trait FromBytes {
    type Output;

    fn from_value(value: rmpv::Value) -> ToBytesResult<Self::Output>;
    fn from_bytes<R: std::io::Read>(rd: &mut R) -> ToBytesResult<Self::Output> {
        let value = read_value(rd)?;
        Self::from_value(value)
    }
}

pub fn read_ns_payload<'a, R: std::io::Read>(
    rd: &'a mut R,
    expected_namespace: &str,
    expected_id: i64,
) -> ToBytesResult<Vec<u8>> {
    let ext_val = rmpv::decode::read_value(rd)?;
    if let rmpv::Value::Ext(type_id, data) = ext_val {
        if type_id != crate::CUSTOM_TYPE_EXT {
            return Err(crate::error::Error::UnexpectedValue(rmpv::Value::String(
                format!(
                    "Expected ext type id '{}', got '{}'",
                    crate::CUSTOM_TYPE_EXT,
                    type_id
                )
                .into(),
            )));
        }
        let mut cursor = std::io::Cursor::new(data);
        let ns_name_utf_raw: rmpv::Utf8String =
            rmpv::decode::read_value(&mut cursor)?.try_into()?;
        let ns_name: &str = ns_name_utf_raw.as_str().ok_or_else(|| {
            crate::error::Error::UnexpectedValue(rmpv::Value::String(
                "Namespace name is not valid UTF-8".into(),
            ))
        })?;
        if ns_name != expected_namespace {
            return Err(crate::error::Error::UnexpectedValue(rmpv::Value::String(
                format!(
                    "Expected namespace '{}', got '{}'",
                    expected_namespace, ns_name
                )
                .into(),
            )));
        }
        let value_id: u64 = rmpv::decode::read_value(&mut cursor)?.try_into()?;
        if value_id != expected_id as u64 {
            return Err(crate::error::Error::UnexpectedValue(rmpv::Value::String(
                format!("Expected id '{}', got '{}'", expected_id, value_id).into(),
            )));
        }
        let pos = cursor.position() as usize;
        Ok(cursor.into_inner()[pos..].to_vec())
    } else {
        Err(crate::error::Error::UnexpectedValue(rmpv::Value::String(
            "Expected ext value".into(),
        )))
    }
}

macro_rules! impl_primitive_decode {
    ($t:ty, $inter:ty) => {
        impl FromBytes for $t {
            type Output = $t;
            fn from_value(value: rmpv::Value) -> ToBytesResult<Self::Output> {
                let inter: $inter = <$inter>::try_from(value)?;
                Ok(inter as Self::Output)
            }
        }
    };
}

impl_primitive_decode!(bool, bool);
impl_primitive_decode!(u8, u64);
impl_primitive_decode!(u16, u64);
impl_primitive_decode!(u32, u64);
impl_primitive_decode!(u64, u64);
impl_primitive_decode!(usize, u64);
impl_primitive_decode!(i8, i64);
impl_primitive_decode!(i16, i64);
impl_primitive_decode!(i32, i64);
impl_primitive_decode!(i64, i64);
impl_primitive_decode!(isize, i64);

impl_primitive_decode!(f32, f32);
impl_primitive_decode!(f64, f64);
impl_primitive_decode!(String, String);

#[derive(Debug, PartialEq)]
pub struct Bytes(pub Vec<u8>);

impl FromBytes for Bytes {
    type Output = Bytes;

    fn from_value(value: rmpv::Value) -> ToBytesResult<Self::Output> {
        let vec = Vec::<u8>::try_from(value)?;
        Ok(Bytes(vec))
    }
}

impl<T> FromBytes for Vec<T>
where
    T: FromBytes<Output = T>,
{
    type Output = Vec<T>;

    fn from_value(value: rmpv::Value) -> ToBytesResult<Self::Output> {
        let vec = Vec::<rmpv::Value>::try_from(value)?;

        Ok(vec
            .into_iter()
            .map(|item| T::from_value(item))
            .collect::<ToBytesResult<Vec<T>>>()?)
    }
}

impl<T, U> FromBytes for std::collections::HashMap<T, U>
where
    T: FromBytes<Output = T> + std::hash::Hash + Eq,
    U: FromBytes<Output = U>,
{
    type Output = std::collections::HashMap<T, U>;

    fn from_value(value: rmpv::Value) -> ToBytesResult<Self::Output> {
        let values = Vec::<(rmpv::Value, rmpv::Value)>::try_from(value)?;

        let mut result = std::collections::HashMap::new();
        for (key, val) in values.into_iter() {
            let k = T::from_value(key)?;
            let v = U::from_value(val)?;
            result.insert(k, v);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use compose_idents::compose;
    use rstest::rstest;

    macro_rules! core_type_value {
        ($ty:ty, $expected:expr, $value:expr) => {
            compose!(
                test_name =
                    snake_case(concat(test_decoding, _, normalize($expected), hash($value))),
                {
                    #[rstest]
                    fn test_name() {
                        let actual = <$ty>::from_bytes(&mut &$value[..]).unwrap();
                        let expected = $expected;
                        assert_eq!(actual, expected);
                    }
                }
            );
        };
    }

    core_type_value!(u8, 1u8, vec![1]);
    core_type_value!(u16, 1u16, vec![1]);
    core_type_value!(u32, 1u32, vec![1]);
    core_type_value!(u64, 1u64, vec![1]);
    core_type_value!(usize, 1usize, vec![1]);
    core_type_value!(i8, 42i8, vec![42]);
    core_type_value!(i16, 42i16, vec![42]);
    core_type_value!(i32, 42i32, vec![42]);
    core_type_value!(i64, 42i64, vec![42]);
    core_type_value!(isize, 42isize, vec![42]);

    core_type_value!(u8, 127u8, vec![127]);
    core_type_value!(u8, 128u8, vec![0xcc, 128]);
    core_type_value!(u16, 128u16, vec![0xcc, 128]);
    core_type_value!(u16, 256u16, vec![0xcd, 1, 0]);
    core_type_value!(u16, 65535u16, vec![0xcd, 255, 255]);
    core_type_value!(u32, 65536u32, vec![0xce, 0, 1, 0, 0]);

    core_type_value!(i8, -1i8, vec![0xff]);
    core_type_value!(i8, -32i8, vec![0xe0]);
    core_type_value!(i8, -33i8, vec![0xd0, 223]);

    core_type_value!(bool, false, vec![0xc2]);
    core_type_value!(bool, true, vec![0xc3]);

    core_type_value!(f32, 3.14f32, vec![0xca, 0x40, 0x48, 0xf5, 0xc3]);
    core_type_value!(
        f64,
        3.14f64,
        vec![0xcb, 0x40, 0x09, 0x1e, 0xb8, 0x51, 0xeb, 0x85, 0x1f]
    );

    core_type_value!(String, "hello", vec![0xa5, 0x68, 0x65, 0x6c, 0x6c, 0x6f]);
    core_type_value!(
        super::Bytes,
        super::Bytes(Vec::from(b"hello")),
        vec![0xc4, 0x05, 0x68, 0x65, 0x6c, 0x6c, 0x6f]
    );
    core_type_value!(Vec<u8>, vec![1u8, 2u8, 3u8], vec![0x93, 0x01, 0x02, 0x03]);

    #[rstest]
    fn test_decoding_hashmap() {
        let value1: Vec<u8> = vec![0b10000010, 0x01, 0x02, 0x03, 0x04]; // {1: 2, 3: 4}
        let value2: Vec<u8> = vec![0b10000010, 0x03, 0x04, 0x01, 0x02]; // {3: 4, 1: 2}

        let expected = {
            let mut map = std::collections::HashMap::new();
            map.insert(1, 2);
            map.insert(3, 4);
            map
        };

        let actual1 = std::collections::HashMap::<u8, u8>::from_bytes(&mut &value1[..]).unwrap();
        let actual2 = std::collections::HashMap::<u8, u8>::from_bytes(&mut &value2[..]).unwrap();

        assert!(
            actual1 == expected,
            "Expected: {:?}, Actual: {:?}",
            expected,
            actual1
        );
        assert!(
            actual2 == expected,
            "Expected: {:?}, Actual: {:?}",
            expected,
            actual2
        );
    }

    #[cfg(feature = "derive")]
    mod derive_tests {
        use super::*;
        use crate::encode::ToBytes;
        use crate::ToBytesResult;

        #[derive(crate::ToBytesDict, crate::FromBytesDict, Debug, PartialEq)]
        struct Person {
            name: String,
            age: u32,
        }

        #[derive(crate::ToBytesDict, crate::FromBytesDict, Debug, PartialEq)]
        struct Point(i32, i32);

        #[derive(crate::ToBytesDict, crate::FromBytesDict, Debug, PartialEq)]
        struct Unit;

        #[rstest]
        fn test_derive_named_struct_round_trip() {
            let person = Person {
                name: "Alice".to_string(),
                age: 30,
            };

            let mut buf = Vec::new();
            person.to_bytes(&mut buf).unwrap();

            let decoded = Person::from_bytes(&mut &buf[..]).unwrap();
            assert_eq!(person, decoded);
        }

        #[rstest]
        fn test_derive_tuple_struct_round_trip() {
            let point = Point(10, 20);

            let mut buf = Vec::new();
            point.to_bytes(&mut buf).unwrap();

            let decoded = Point::from_bytes(&mut &buf[..]).unwrap();
            assert_eq!(point, decoded);
        }

        #[rstest]
        fn test_derive_unit_struct_round_trip() {
            let unit = Unit;

            let mut buf = Vec::new();
            unit.to_bytes(&mut buf).unwrap();

            let decoded = Unit::from_bytes(&mut &buf[..]).unwrap();
            assert_eq!(unit, decoded);
        }
    }
}

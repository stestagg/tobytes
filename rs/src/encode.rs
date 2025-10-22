use crate::ToBytesResult;
use rmpv::encode::write_value_ref;
use std::io::Write;

pub trait ToBytes {
    fn to_bytes<W: Write>(&self, wr: &mut W) -> ToBytesResult<()>;
}

macro_rules! impl_primitive_encode {
    ($t:ty) => {
        impl ToBytes for $t {
            fn to_bytes<W: Write>(&self, wr: &mut W) -> ToBytesResult<()> {
                let value: rmpv::ValueRef = (*self).into();
                write_value_ref(wr, &value)?;
                Ok(())
            }
        }
    };
}
macro_rules! impl_primitive_encode_ref {
    ($t:ty) => {
        impl ToBytes for &$t {
            fn to_bytes<W: Write>(&self, wr: &mut W) -> ToBytesResult<()> {
                let value: rmpv::ValueRef = (*self).into();
                write_value_ref(wr, &value)?;
                Ok(())
            }
        }
    };
}

impl_primitive_encode_ref! {[u8]}
impl_primitive_encode_ref! {str}
impl_primitive_encode! {f32}
impl_primitive_encode! {f64}
impl_primitive_encode! {i16}
impl_primitive_encode! {i32}
impl_primitive_encode! {i64}
impl_primitive_encode! {i8}
impl_primitive_encode! {isize}
impl_primitive_encode! {u16}
impl_primitive_encode! {u32}
impl_primitive_encode! {u64}
impl_primitive_encode! {u8}
impl_primitive_encode! {usize}

impl ToBytes for bool {
    fn to_bytes<W: Write>(&self, wr: &mut W) -> ToBytesResult<()> {
        let value: rmpv::ValueRef = rmpv::ValueRef::Boolean(*self);
        write_value_ref(wr, &value)?;
        Ok(())
    }
}

impl ToBytes for String {
    fn to_bytes<W: Write>(&self, wr: &mut W) -> ToBytesResult<()> {
        let value: rmpv::ValueRef = (*self).as_str().into();
        write_value_ref(wr, &value)?;
        Ok(())
    }
}

impl<T: ToBytes> ToBytes for Vec<T> {
    fn to_bytes<W: Write>(&self, wr: &mut W) -> ToBytesResult<()> {
        let len = self.len() as u32;
        rmp::encode::write_array_len(wr, len)?;
        for item in self {
            item.to_bytes(wr)?;
        }
        Ok(())
    }
}

impl<K: ToBytes, V: ToBytes> ToBytes for std::collections::HashMap<K, V> {
    fn to_bytes<W: Write>(&self, wr: &mut W) -> ToBytesResult<()> {
        let len = self.len() as u32;
        rmp::encode::write_map_len(wr, len)?;
        for (key, value) in self {
            key.to_bytes(wr)?;
            value.to_bytes(wr)?;
        }
        Ok(())
    }
}

impl<const S: usize> ToBytes for &[u8; S] {
    fn to_bytes<W: Write>(&self, wr: &mut W) -> ToBytesResult<()> {
        let value: rmpv::ValueRef = rmpv::ValueRef::Binary(self.as_ref());
        write_value_ref(wr, &value)?;
        Ok(())
    }
}

pub struct NamespaceEncodedValue {
    pub namespace: &'static str,
    pub id: u32,
    pub value: Vec<u8>,
}

const CUSTOM_TYPE_EXT: i8 = 8;

impl ToBytes for NamespaceEncodedValue {
    fn to_bytes<W: Write>(&self, wr: &mut W) -> ToBytesResult<()> {
        let mut pfx_buf = Vec::with_capacity(self.namespace.len() + 2 + 9);
        rmp::encode::write_str(&mut pfx_buf, self.namespace)?;
        rmp::encode::write_sint(&mut pfx_buf, self.id as i64)?;
        let total_len = pfx_buf.len() + self.value.len();
        rmp::encode::write_ext_meta(wr, total_len as u32, CUSTOM_TYPE_EXT)?;
        wr.write_all(&pfx_buf)?;
        wr.write_all(&self.value)?;
        Ok(())
    }
}

pub struct NamespaceValue<T: ToBytes> {
    namespace: &'static str,
    id: u32,
    value: T,
}

impl<T: ToBytes> ToBytes for NamespaceValue<T> {
    fn to_bytes<W: Write>(&self, wr: &mut W) -> ToBytesResult<()> {
        let mut buf = Vec::with_capacity(self.namespace.len() + 2 + 9 + 21);
        rmp::encode::write_str(&mut buf, self.namespace)?;
        rmp::encode::write_sint(&mut buf, self.id as i64)?;
        self.value.to_bytes(&mut buf)?;
        rmp::encode::write_ext_meta(wr, buf.len() as u32, CUSTOM_TYPE_EXT)?;
        wr.write_all(&buf)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use compose_idents::compose;
    use rstest::rstest;

    macro_rules! core_type_value {
        ($value:expr, $expected:expr) => {
            compose!(
                test_name = snake_case(concat(test_encoding, _, normalize($value), hash($value))),
                {
                    #[rstest]
                    fn test_name() {
                        let buf: &mut Vec<u8> = &mut Vec::new();
                        ($value).to_bytes(buf).unwrap();
                        let actuals: Vec<u8> = buf.clone();
                        let expecteds: Vec<u8> = $expected;
                        assert_eq!(actuals, expecteds);
                    }
                }
            );
        };
    }

    core_type_value!(1u8, vec![1]);
    core_type_value!(1u16, vec![1]);
    core_type_value!(1u32, vec![1]);
    core_type_value!(1u64, vec![1]);
    core_type_value!(1usize, vec![1]);
    core_type_value!(42i8, vec![42]);
    core_type_value!(42i16, vec![42]);
    core_type_value!(42i32, vec![42]);
    core_type_value!(42i64, vec![42]);
    core_type_value!(42isize, vec![42]);

    core_type_value!(127u8, vec![127]);
    core_type_value!(128u8, vec![0xcc, 128]);
    core_type_value!(128u16, vec![0xcc, 128]);
    core_type_value!(256u16, vec![0xcd, 1, 0]);
    core_type_value!(65535u16, vec![0xcd, 255, 255]);
    core_type_value!(65536u32, vec![0xce, 0, 1, 0, 0]);

    core_type_value!(-1i8, vec![0xff]);
    core_type_value!(-32i8, vec![0xe0]);
    core_type_value!(-33i8, vec![0xd0, 223]);

    core_type_value!(false, vec![0xc2]);
    core_type_value!(true, vec![0xc3]);

    core_type_value!(3.14f32, vec![0xca, 0x40, 0x48, 0xf5, 0xc3]);
    core_type_value!(
        3.14f64,
        vec![0xcb, 0x40, 0x09, 0x1e, 0xb8, 0x51, 0xeb, 0x85, 0x1f]
    );

    core_type_value!("hello", vec![0xa5, 0x68, 0x65, 0x6c, 0x6c, 0x6f]);
    core_type_value!(
        "hello".to_string(),
        vec![0xa5, 0x68, 0x65, 0x6c, 0x6c, 0x6f]
    );
    core_type_value!(b"hello", vec![0xc4, 0x05, 0x68, 0x65, 0x6c, 0x6c, 0x6f]);

    core_type_value!(vec![1u8, 2u8, 3u8], vec![0x93, 0x01, 0x02, 0x03]);
    #[rstest]
    fn test_encoding_hashmap() {
        let buf: &mut Vec<u8> = &mut Vec::new();
        let mut map = std::collections::HashMap::new();
        map.insert(1, 2);
        map.insert(3, 4);
        map.to_bytes(buf).unwrap();
        let actuals: Vec<u8> = buf.clone();

        // HashMap iteration order is undefined, so check for both possible orderings
        let expected_order1: Vec<u8> = vec![0b10000010, 0x01, 0x02, 0x03, 0x04]; // {1: 2, 3: 4}
        let expected_order2: Vec<u8> = vec![0b10000010, 0x03, 0x04, 0x01, 0x02]; // {3: 4, 1: 2}

        assert!(
            actuals == expected_order1 || actuals == expected_order2,
            "Expected one of {:?} or {:?}, but got {:?}",
            expected_order1,
            expected_order2,
            actuals
        );
    }

    #[cfg(feature = "derive")]
    mod derive_tests {
        use super::*;
        use crate::ToBytesResult;

        #[derive(crate::ToBytesDict)]
        struct Person {
            name: String,
            age: u32,
        }

        #[derive(crate::ToBytesDict)]
        struct Point(i32, i32);

        #[derive(crate::ToBytesDict)]
        struct Unit;

        #[derive(crate::ToBytesDict)]
        struct NestedStruct {
            point: Point,
            data: Vec<u32>,
        }

        #[rstest]
        fn test_derive_named_struct() {
            let person = Person {
                name: "Alice".to_string(),
                age: 30,
            };
            let buf: &mut Vec<u8> = &mut Vec::new();
            person.to_bytes(buf).unwrap();

            // Should encode as a map with 2 entries
            assert_eq!(buf[0], 0b10000010); // fixmap with 2 entries

            // Decode to verify structure
            let decoded = rmpv::decode::read_value(&mut &buf[..]).unwrap();
            if let rmpv::Value::Map(map) = decoded {
                assert_eq!(map.len(), 2);
                // Check that we have the expected keys
                let keys: Vec<String> = map
                    .iter()
                    .map(|(k, _)| k.as_str().unwrap().to_string())
                    .collect();
                assert!(keys.contains(&"name".to_string()));
                assert!(keys.contains(&"age".to_string()));
            } else {
                panic!("Expected a map");
            }
        }

        #[rstest]
        fn test_derive_tuple_struct() {
            let point = Point(10, 20);
            let buf: &mut Vec<u8> = &mut Vec::new();
            point.to_bytes(buf).unwrap();

            // Should encode as an array with 2 elements
            assert_eq!(buf[0], 0b10010010); // fixarray with 2 elements
            assert_eq!(buf[1], 10);
            assert_eq!(buf[2], 20);
        }

        #[rstest]
        fn test_derive_unit_struct() {
            let unit = Unit;
            let buf: &mut Vec<u8> = &mut Vec::new();
            unit.to_bytes(buf).unwrap();

            // Should encode as an empty array
            assert_eq!(buf[0], 0b10010000); // fixarray with 0 elements
        }

        #[rstest]
        fn test_derive_nested_struct() {
            let nested = NestedStruct {
                point: Point(5, 15),
                data: vec![1, 2, 3],
            };
            let buf: &mut Vec<u8> = &mut Vec::new();
            nested.to_bytes(buf).unwrap();

            // Should encode as a map with 2 entries
            assert_eq!(buf[0], 0b10000010); // fixmap with 2 entries

            // Decode to verify structure
            let decoded = rmpv::decode::read_value(&mut &buf[..]).unwrap();
            if let rmpv::Value::Map(map) = decoded {
                assert_eq!(map.len(), 2);
            } else {
                panic!("Expected a map");
            }
        }
    }
}

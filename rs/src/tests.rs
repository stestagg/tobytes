use super::*;

fn decode_roundtrip(object: &Object) -> Object {
    let mut codec = Codec::default();
    let encoded = codec.dumps(object).expect("encode");
    codec.loads(&encoded).expect("decode")
}

#[test]
fn encode_decode_primitives() {
    let object = Object::map(vec![
        (Object::from("name"), Object::from("tobytes")),
        (Object::from("count"), Object::from(3_u64)),
        (Object::from("active"), Object::from(true)),
    ]);

    let decoded = decode_roundtrip(&object);
    assert_eq!(decoded, object);
}

#[test]
fn encode_with_intern_table() {
    let shared = Object::array(vec![
        Object::from("alpha"),
        Object::from("beta"),
        Object::from("gamma"),
    ]);
    let intern = Object::Intern(InternValue::new(shared.clone()));

    let object = Object::array(vec![
        intern.clone(),
        Object::map(vec![
            (
                Object::from("items"),
                Object::array(vec![intern.clone(), Object::from("delta")]),
            ),
            (Object::from("repeat"), intern.clone()),
        ]),
        shared.clone(),
    ]);

    // After encoding and decoding, Intern wrappers should be resolved to their actual values
    let expected = Object::array(vec![
        shared.clone(),
        Object::map(vec![
            (
                Object::from("items"),
                Object::array(vec![shared.clone(), Object::from("delta")]),
            ),
            (Object::from("repeat"), shared.clone()),
        ]),
        shared,
    ]);

    let mut codec = Codec::default();
    let encoded = codec.dumps(&object).expect("encode");
    let decoded = codec.loads(&encoded).expect("decode");

    assert_eq!(decoded, expected);
}

#[test]
fn decode_intern_forward_reference_fails() {
    let mut codec = Codec::default();
    // Construct an invalid message: intern table referencing a future entry.
    let mut payload = Vec::new();
    // interned objects array with one element which references index 1 (forward)
    {
        let mut entries_buf = Vec::new();
        rmp::encode::write_array_len(&mut entries_buf, 1).unwrap();
        // entry 0 contains a reference to index 1 (forward)
        let mut ref_buf = Vec::new();
        rmp::encode::write_uint(&mut ref_buf, 1).unwrap();
        rmp::encode::write_ext_meta(
            &mut entries_buf,
            ref_buf.len() as u32,
            crate::intern::INTERN_TABLE_EXT,
        )
        .unwrap();
        entries_buf.extend_from_slice(&ref_buf);
        payload.extend_from_slice(&entries_buf);
    }
    // data section (nil)
    rmp::encode::write_nil(&mut payload).unwrap();

    let mut message = Vec::new();
    rmp::encode::write_ext_meta(
        &mut message,
        payload.len() as u32,
        crate::intern::INTERN_TABLE_EXT,
    )
    .unwrap();
    message.extend_from_slice(&payload);

    let result = codec.loads(&message);
    assert!(matches!(result, Err(Error::ForwardInternReference { .. })));
}

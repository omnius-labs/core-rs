#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use testresult::TestResult;

    use crate::{FieldType, RocketPackBytesDecoder, RocketPackBytesEncoder, RocketPackDecoder, RocketPackDecoderError, RocketPackEncoder};

    fn compose(major: u8, info: u8) -> u8 {
        (major << 5) | info
    }

    macro_rules! encode_test {
        ($name:ident, $value:expr, $bytes:expr) => {{
            let mut buf = Vec::new();
            let mut encoder = RocketPackBytesEncoder::new(&mut buf);
            encoder.$name($value)?;
            assert_eq!(buf.as_slice(), $bytes, "encode hex: {}", hex::encode($bytes));
        }};
    }

    macro_rules! decode_test {
        ($name:ident, $bytes:expr, $value:expr) => {{
            let mut decoder = RocketPackBytesDecoder::new($bytes);
            let decoded = decoder.$name()?;
            assert_eq!(decoded, $value, "decode hex: {}", hex::encode($bytes));
        }};
    }

    #[test]
    fn normal_bool_test() -> TestResult {
        let cases: Vec<(Vec<u8>, bool)> = vec![
            (vec![compose(7, 20)], false),
            (vec![compose(7, 21)], true), //
        ];

        for (bytes, value) in cases {
            encode_test!(write_bool, value, &bytes);
            decode_test!(read_bool, &bytes, value);
        }

        Ok(())
    }

    #[test]
    fn normal_u8_test() -> TestResult {
        let cases: Vec<(Vec<u8>, u8)> = vec![
            (vec![compose(0, 0)], 0),
            (vec![compose(0, 23)], 23),
            (vec![compose(0, 24), 24], 24),
            (vec![compose(0, 24), 255], u8::MAX),
        ];

        for (bytes, value) in cases {
            encode_test!(write_u8, value, &bytes);
            decode_test!(read_u8, &bytes, value);
        }

        Ok(())
    }

    #[test]
    fn normal_u16_test() -> TestResult {
        let cases: Vec<(Vec<u8>, u16)> = vec![
            (vec![compose(0, 0)], 0),
            (vec![compose(0, 23)], 23),
            (vec![compose(0, 24), 24], 24),
            (vec![compose(0, 24), 255], u8::MAX as u16),
            (vec![compose(0, 25), 255, 255], u16::MAX),
        ];

        for (bytes, value) in cases {
            encode_test!(write_u16, value, &bytes);
            decode_test!(read_u16, &bytes, value);
        }

        Ok(())
    }

    #[test]
    fn normal_u32_test() -> TestResult {
        let cases: Vec<(Vec<u8>, u32)> = vec![
            (vec![compose(0, 0)], 0),
            (vec![compose(0, 23)], 23),
            (vec![compose(0, 24), 24], 24),
            (vec![compose(0, 24), 255], u8::MAX as u32),
            (vec![compose(0, 25), 255, 255], u16::MAX as u32),
            (vec![compose(0, 26), 255, 255, 255, 255], u32::MAX),
        ];

        for (bytes, value) in cases {
            encode_test!(write_u32, value, &bytes);
            decode_test!(read_u32, &bytes, value);
        }

        Ok(())
    }

    #[test]
    fn normal_u64_test() -> TestResult {
        let cases: Vec<(Vec<u8>, u64)> = vec![
            (vec![compose(0, 0)], 0),
            (vec![compose(0, 23)], 23),
            (vec![compose(0, 24), 24], 24),
            (vec![compose(0, 24), 255], u8::MAX as u64),
            (vec![compose(0, 25), 255, 255], u16::MAX as u64),
            (vec![compose(0, 26), 255, 255, 255, 255], u32::MAX as u64),
            (vec![compose(0, 27), 255, 255, 255, 255, 255, 255, 255, 255], u64::MAX),
        ];

        for (bytes, value) in cases {
            encode_test!(write_u64, value, &bytes);
            decode_test!(read_u64, &bytes, value);
        }

        Ok(())
    }

    #[test]
    fn normal_i8_test() -> TestResult {
        let cases: Vec<(Vec<u8>, i8)> = vec![
            (vec![compose(0, 0)], 0),
            (vec![compose(0, 23)], 23),
            (vec![compose(0, 24), 24], 24),
            (vec![compose(0, 24), 127], i8::MAX),
            (vec![compose(1, 0)], -1),
            (vec![compose(1, 23)], -24),
            (vec![compose(1, 24), 24], -25),
            (vec![compose(1, 24), 127], i8::MIN),
        ];

        for (bytes, value) in cases {
            encode_test!(write_i8, value, &bytes);
            decode_test!(read_i8, &bytes, value);
        }

        Ok(())
    }

    #[test]
    fn normal_i16_test() -> TestResult {
        let cases: Vec<(Vec<u8>, i16)> = vec![
            (vec![compose(0, 0)], 0),
            (vec![compose(0, 23)], 23),
            (vec![compose(0, 24), 24], 24),
            (vec![compose(0, 24), 255], u8::MAX as i16),
            (vec![compose(0, 25), 127, 255], i16::MAX),
            (vec![compose(1, 0)], -1),
            (vec![compose(1, 23)], -24),
            (vec![compose(1, 24), 24], -25),
            (vec![compose(1, 24), 255], -((u8::MAX as i16) + 1)),
            (vec![compose(1, 25), 1, 0], -((u8::MAX as i16) + 2)),
            (vec![compose(1, 25), 127, 255], i16::MIN),
        ];

        for (bytes, value) in cases {
            encode_test!(write_i16, value, &bytes);
            decode_test!(read_i16, &bytes, value);
        }

        Ok(())
    }

    #[test]
    fn normal_i32_test() -> TestResult {
        let cases: Vec<(Vec<u8>, i32)> = vec![
            (vec![compose(0, 0)], 0),
            (vec![compose(0, 23)], 23),
            (vec![compose(0, 24), 24], 24),
            (vec![compose(0, 24), 255], u8::MAX as i32),
            (vec![compose(0, 25), 255, 255], u16::MAX as i32),
            (vec![compose(0, 26), 127, 255, 255, 255], i32::MAX),
            (vec![compose(1, 0)], -1),
            (vec![compose(1, 23)], -24),
            (vec![compose(1, 24), 24], -25),
            (vec![compose(1, 24), 255], -((u8::MAX as i32) + 1)),
            (vec![compose(1, 25), 1, 0], -((u8::MAX as i32) + 2)),
            (vec![compose(1, 25), 255, 255], -((u16::MAX as i32) + 1)),
            (vec![compose(1, 26), 0, 1, 0, 0], -((u16::MAX as i32) + 2)),
            (vec![compose(1, 26), 127, 255, 255, 255], i32::MIN),
        ];

        for (bytes, value) in cases {
            encode_test!(write_i32, value, &bytes);
            decode_test!(read_i32, &bytes, value);
        }

        Ok(())
    }

    #[test]
    fn normal_i64_test() -> TestResult {
        let cases: Vec<(Vec<u8>, i64)> = vec![
            (vec![compose(0, 0)], 0),
            (vec![compose(0, 23)], 23),
            (vec![compose(0, 24), 24], 24),
            (vec![compose(0, 24), 255], u8::MAX as i64),
            (vec![compose(0, 25), 255, 255], u16::MAX as i64),
            (vec![compose(0, 26), 255, 255, 255, 255], u32::MAX as i64),
            (vec![compose(0, 27), 127, 255, 255, 255, 255, 255, 255, 255], i64::MAX),
            (vec![compose(1, 0)], -1),
            (vec![compose(1, 23)], -24),
            (vec![compose(1, 24), 24], -25),
            (vec![compose(1, 24), 255], -((u8::MAX as i64) + 1)),
            (vec![compose(1, 25), 1, 0], -((u8::MAX as i64) + 2)),
            (vec![compose(1, 25), 255, 255], -((u16::MAX as i64) + 1)),
            (vec![compose(1, 26), 0, 1, 0, 0], -((u16::MAX as i64) + 2)),
            (vec![compose(1, 26), 255, 255, 255, 255], -((u32::MAX as i64) + 1)),
            (vec![compose(1, 27), 0, 0, 0, 1, 0, 0, 0, 0], -((u32::MAX as i64) + 2)),
            (vec![compose(1, 27), 127, 255, 255, 255, 255, 255, 255, 255], i64::MIN),
        ];

        for (bytes, value) in cases {
            encode_test!(write_i64, value, &bytes);
            decode_test!(read_i64, &bytes, value);
        }

        Ok(())
    }

    #[test]
    fn normal_f32_test() -> TestResult {
        let cases: Vec<(Vec<u8>, f32)> = vec![
            (vec![compose(7, 26), 0, 0, 0, 0], 0.0), //
        ];

        for (bytes, value) in cases {
            encode_test!(write_f32, value, &bytes);
            decode_test!(read_f32, &bytes, value);
        }

        Ok(())
    }

    #[test]
    fn normal_f64_test() -> TestResult {
        let cases: Vec<(Vec<u8>, f64)> = vec![
            (vec![compose(7, 27), 0, 0, 0, 0, 0, 0, 0, 0], 0.0), //
        ];

        for (bytes, value) in cases {
            encode_test!(write_f64, value, &bytes);
            decode_test!(read_f64, &bytes, value);
        }

        Ok(())
    }

    #[test]
    fn normal_bytes_test() -> TestResult {
        let cases: Vec<(Vec<u8>, Vec<u8>)> = vec![
            (vec![compose(2, 0)], vec![]),
            (vec![compose(2, 1), 0], vec![0]),
            (([vec![compose(2, 23)], vec![0; 23]].concat()), vec![0; 23]),
            (([vec![compose(2, 24), 24], vec![0; 24]].concat()), vec![0; 24]),
        ];

        for (bytes, value) in cases {
            encode_test!(write_bytes, &value, &bytes);
            decode_test!(read_bytes_vec, &bytes, value);
        }

        Ok(())
    }

    #[test]
    fn normal_string_test() -> TestResult {
        let cases: Vec<(Vec<u8>, String)> = vec![
            (vec![compose(3, 0)], "".to_string()),
            (vec![compose(3, 6), 65, 65, 66, 66, 67, 67], "AABBCC".to_string()), //
        ];

        for (bytes, value) in cases {
            encode_test!(write_string, &value, &bytes);
            decode_test!(read_string, &bytes, value);
        }

        Ok(())
    }

    #[test]
    fn normal_array_test() -> TestResult {
        let cases: Vec<(Vec<u8>, usize)> = vec![
            (vec![compose(4, 1)], 1), //
        ];

        for (bytes, value) in cases {
            encode_test!(write_array, value, &bytes);
            decode_test!(read_array, &bytes, value as u64);
        }

        Ok(())
    }

    #[test]
    fn normal_map_test() -> TestResult {
        let cases: Vec<(Vec<u8>, usize)> = vec![
            (vec![compose(5, 1)], 1), //
        ];

        for (bytes, value) in cases {
            encode_test!(write_map, value, &bytes);
            decode_test!(read_map, &bytes, value as u64);
        }

        Ok(())
    }

    #[test]
    fn normal_raw_len_bytes_test() -> TestResult {
        let cases: Vec<(Vec<u8>, u64, u8)> = vec![
            (vec![compose(0, 0)], 0, 0),
            (vec![compose(0, 23)], 23, 23),
            (vec![compose(0, 24), 24], 24, 24),
            (vec![compose(0, 24), 255], u8::MAX as u64, 24),
            (vec![compose(0, 25), 255, 255], u16::MAX as u64, 25),
            (vec![compose(0, 26), 255, 255, 255, 255], u32::MAX as u64, 26),
            (vec![compose(0, 27), 255, 255, 255, 255, 255, 255, 255, 255], u64::MAX, 27),
        ];

        for (bytes, value, info) in cases {
            let mut buf = Vec::new();
            let mut encoder = RocketPackBytesEncoder::new(&mut buf);
            encoder.write_raw_len(0, value as usize)?;
            assert_eq!(buf.as_slice(), bytes, "encode hex: {}", hex::encode(&bytes));

            let mut decoder = RocketPackBytesDecoder::new(&bytes[1..]);
            let decoded = decoder.read_raw_len(info)?;
            assert_eq!(decoded, Some(value), "decode hex: {}", hex::encode(&bytes));
        }

        Ok(())
    }

    #[test]
    fn normal_decoder_type_of_test() -> TestResult {
        let cases: Vec<(Vec<u8>, FieldType)> = vec![
            (vec![compose(0, 0)], FieldType::U8),
            (vec![compose(0, 24)], FieldType::U8),
            (vec![compose(0, 25)], FieldType::U16),
            (vec![compose(0, 26)], FieldType::U32),
            (vec![compose(0, 27)], FieldType::U64),
            (vec![compose(1, 0)], FieldType::U8),
            (vec![compose(1, 24), 0], FieldType::I8),
            (vec![compose(1, 25), 0], FieldType::I16),
            (vec![compose(1, 26), 0], FieldType::I32),
            (vec![compose(1, 27), 0], FieldType::I64),
            (vec![compose(1, 24), 0x80], FieldType::I16),
            (vec![compose(1, 25), 0x80], FieldType::I32),
            (vec![compose(1, 26), 0x80], FieldType::I64),
            (vec![compose(2, 0)], FieldType::Bytes),
            (vec![compose(3, 0)], FieldType::String),
            (vec![compose(4, 0)], FieldType::Array),
            (vec![compose(5, 0)], FieldType::Map),
            (vec![compose(7, 20)], FieldType::Bool),
            (vec![compose(7, 21)], FieldType::Bool),
            (vec![compose(7, 25)], FieldType::F16),
            (vec![compose(7, 26)], FieldType::F32),
            (vec![compose(7, 27)], FieldType::F64),
            (vec![compose(7, 31)], FieldType::Unknown { major: 7, info: 31 }),
        ];

        for (bytes, typ) in cases {
            let decoder = RocketPackBytesDecoder::new(&bytes);
            assert_eq!(decoder.current_type()?, typ, "decode hex: {}", hex::encode(&bytes));
        }

        Ok(())
    }

    #[test]
    fn normal_decoder_skip_field_test() -> TestResult {
        let p0 = true;
        let p1 = 1;
        let p2 = 2;
        let p3 = 3;
        let p4 = 4;
        let p5 = 5;
        let p6 = 6;
        let p7 = 7;
        let p8 = 8;
        let p9 = 9.5;
        let p10 = 10.5;
        let p11 = vec![0xAA, 0xBB, 0xCC];
        let p12 = "test".to_string();
        let p13 = ["test_0".to_string(), "test_1".to_string()];
        let p14 = BTreeMap::from([
            (0_u32, "test_value_0".to_string()),
            (1_u32, "test_value_1".to_string()),
            (2_u32, "test_value_2".to_string()),
        ]);

        let mut buf = Vec::new();
        let mut encoder = RocketPackBytesEncoder::new(&mut buf);

        encoder.write_bool(p0)?;
        encoder.write_u8(p1)?;
        encoder.write_u16(p2)?;
        encoder.write_u32(p3)?;
        encoder.write_u64(p4)?;
        encoder.write_i8(p5)?;
        encoder.write_i16(p6)?;
        encoder.write_i32(p7)?;
        encoder.write_i64(p8)?;
        encoder.write_f32(p9)?;
        encoder.write_f64(p10)?;
        encoder.write_bytes(p11.as_slice())?;
        encoder.write_string(p12.as_str())?;
        encoder.write_array(p13.len())?;
        for v in p13.iter() {
            encoder.write_string(v.as_str())?;
        }
        encoder.write_map(p14.len())?;
        for v in p14.iter() {
            encoder.write_u32(*v.0)?;
            encoder.write_string(v.1.as_str())?;
        }

        let mut decoder = RocketPackBytesDecoder::new(&buf);

        for _ in 0..=14 {
            decoder.skip_field()?;
        }

        assert_eq!(decoder.remaining(), 0);

        Ok(())
    }

    #[test]
    fn truncated_negative_number_reports_eof() -> TestResult {
        let bytes = vec![compose(1, 24)];
        let decoder = RocketPackBytesDecoder::new(&bytes);

        match decoder.current_type() {
            Err(RocketPackDecoderError::UnexpectedEof) => Ok(()),
            other => panic!("expected UnexpectedEof, got {other:?}"),
        }
    }
}

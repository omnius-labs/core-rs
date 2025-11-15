use crate::{RocketPackBytesDecoder, RocketPackBytesEncoder, RocketPackDecoder, RocketPackDecoderError, RocketPackEncoder, RocketPackEncoderError};

#[allow(clippy::len_without_is_empty)]
pub trait RocketPackStruct {
    fn pack(encoder: &mut impl RocketPackEncoder, value: &Self) -> std::result::Result<(), RocketPackEncoderError>;

    fn unpack(decoder: &mut impl RocketPackDecoder) -> std::result::Result<Self, RocketPackDecoderError>
    where
        Self: Sized;

    fn import(bytes: &[u8]) -> std::result::Result<Self, RocketPackDecoderError>
    where
        Self: Sized,
    {
        let mut decoder = RocketPackBytesDecoder::new(bytes);
        Self::unpack(&mut decoder)
    }

    fn export(&self) -> std::result::Result<Vec<u8>, RocketPackEncoderError> {
        let mut bytes: Vec<u8> = Vec::new();
        let mut encoder = RocketPackBytesEncoder::new(&mut bytes);
        Self::pack(&mut encoder, self)?;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, rc::Rc};

    use crate::{RocketPackDecoder, RocketPackDecoderError, RocketPackEncoder, RocketPackEncoderError, RocketPackStruct};
    use testresult::TestResult;

    #[test]
    fn normal_test() -> TestResult {
        let mut value = NormalTestStruct {
            p0: true,
            p1: 1,
            p2: 2,
            p3: 3,
            p4: 4,
            p5: 5,
            p6: 6,
            p7: 7,
            p8: 8,
            p9: 9.5,
            p10: 10.5,
            p11: vec![0xAA, 0xBB, 0xCC],
            p12: "test".to_string(),
            p13: vec!["test_0".to_string(), "test_1".to_string()],
            p14: BTreeMap::from([
                (0_u32, "test_value_0".to_string()),
                (1_u32, "test_value_1".to_string()),
                (2_u32, "test_value_2".to_string()),
            ]),
            p15: None,
        };
        value.p15 = Some(Rc::new(value.clone()));

        let exported = value.export()?;
        let imported = NormalTestStruct::import(exported.as_slice())?;
        assert_normal_struct(&imported, &value);

        Ok(())
    }

    fn assert_normal_struct(actual: &NormalTestStruct, expected: &NormalTestStruct) {
        assert_eq!(actual.p0, expected.p0);
        assert_eq!(actual.p1, expected.p1);
        assert_eq!(actual.p2, expected.p2);
        assert_eq!(actual.p3, expected.p3);
        assert_eq!(actual.p4, expected.p4);
        assert_eq!(actual.p5, expected.p5);
        assert_eq!(actual.p6, expected.p6);
        assert_eq!(actual.p7, expected.p7);
        assert_eq!(actual.p8, expected.p8);
        assert!((actual.p9 - expected.p9).abs() < f32::EPSILON);
        assert!((actual.p10 - expected.p10).abs() < f64::EPSILON);
        assert_eq!(actual.p11, expected.p11);
        assert_eq!(actual.p12, expected.p12);
        assert_eq!(actual.p13, expected.p13);
        assert_eq!(actual.p14, expected.p14);

        match (&actual.p15, &expected.p15) {
            (Some(a), Some(b)) => assert_normal_struct(a.as_ref(), b.as_ref()),
            (None, None) => {}
            _ => panic!("p15 mismatch"),
        }
    }

    #[derive(Debug, Clone)]
    struct NormalTestStruct {
        pub p0: bool,
        pub p1: u8,
        pub p2: u16,
        pub p3: u32,
        pub p4: u64,
        pub p5: i8,
        pub p6: i16,
        pub p7: i32,
        pub p8: i64,
        pub p9: f32,
        pub p10: f64,
        pub p11: Vec<u8>,
        pub p12: String,
        pub p13: Vec<String>,
        pub p14: BTreeMap<u32, String>,
        pub p15: Option<Rc<NormalTestStruct>>,
    }

    impl RocketPackStruct for NormalTestStruct {
        fn pack(encoder: &mut impl RocketPackEncoder, value: &Self) -> std::result::Result<(), RocketPackEncoderError> {
            let mut count = 15;
            if value.p15.is_some() {
                count += 1;
            }

            encoder.write_map(count)?;

            encoder.write_u64(0)?;
            encoder.write_bool(value.p0)?;

            encoder.write_u64(1)?;
            encoder.write_u8(value.p1)?;

            encoder.write_u64(2)?;
            encoder.write_u16(value.p2)?;

            encoder.write_u64(3)?;
            encoder.write_u32(value.p3)?;

            encoder.write_u64(4)?;
            encoder.write_u64(value.p4)?;

            encoder.write_u64(5)?;
            encoder.write_i8(value.p5)?;

            encoder.write_u64(6)?;
            encoder.write_i16(value.p6)?;

            encoder.write_u64(7)?;
            encoder.write_i32(value.p7)?;

            encoder.write_u64(8)?;
            encoder.write_i64(value.p8)?;

            encoder.write_u64(9)?;
            encoder.write_f32(value.p9)?;

            encoder.write_u64(10)?;
            encoder.write_f64(value.p10)?;

            encoder.write_u64(11)?;
            encoder.write_bytes(value.p11.as_slice())?;

            encoder.write_u64(12)?;
            encoder.write_string(value.p12.as_str())?;

            encoder.write_u64(13)?;
            encoder.write_array(value.p13.len())?;
            for v in value.p13.iter() {
                encoder.write_string(v.as_str())?;
            }

            encoder.write_u64(14)?;
            encoder.write_map(value.p14.len())?;
            for v in value.p14.iter() {
                encoder.write_u32(*v.0)?;
                encoder.write_string(v.1.as_str())?;
            }

            if let Some(p15) = &value.p15 {
                encoder.write_u64(15)?;
                encoder.write_struct(p15.as_ref())?;
            }

            Ok(())
        }

        fn unpack(decoder: &mut impl RocketPackDecoder) -> std::result::Result<Self, RocketPackDecoderError>
        where
            Self: Sized,
        {
            let mut p0: Option<bool> = None;
            let mut p1: Option<u8> = None;
            let mut p2: Option<u16> = None;
            let mut p3: Option<u32> = None;
            let mut p4: Option<u64> = None;
            let mut p5: Option<i8> = None;
            let mut p6: Option<i16> = None;
            let mut p7: Option<i32> = None;
            let mut p8: Option<i64> = None;
            let mut p9: Option<f32> = None;
            let mut p10: Option<f64> = None;
            let mut p11: Option<Vec<u8>> = None;
            let mut p12: Option<String> = None;
            let mut p13: Vec<String> = vec![];
            let mut p14: BTreeMap<u32, String> = BTreeMap::new();
            let mut p15: Option<Rc<NormalTestStruct>> = None;

            let count = decoder.read_map()?;

            for _ in 0..count {
                match decoder.read_u64()? {
                    0 => p0 = Some(decoder.read_bool()?),
                    1 => p1 = Some(decoder.read_u8()?),
                    2 => p2 = Some(decoder.read_u16()?),
                    3 => p3 = Some(decoder.read_u32()?),
                    4 => p4 = Some(decoder.read_u64()?),
                    5 => p5 = Some(decoder.read_i8()?),
                    6 => p6 = Some(decoder.read_i16()?),
                    7 => p7 = Some(decoder.read_i32()?),
                    8 => p8 = Some(decoder.read_i64()?),
                    9 => p9 = Some(decoder.read_f32()?),
                    10 => p10 = Some(decoder.read_f64()?),
                    11 => p11 = Some(decoder.read_bytes_vec()?),
                    12 => p12 = Some(decoder.read_string()?),
                    13 => {
                        let count = decoder.read_array()?;
                        for _ in 0..count {
                            p13.push(decoder.read_string()?);
                        }
                    }
                    14 => {
                        let count = decoder.read_map()?;
                        for _ in 0..count {
                            let key = decoder.read_u32()?;
                            let value = decoder.read_string()?;
                            p14.insert(key, value);
                        }
                    }
                    15 => p15 = Some(Rc::new(decoder.read_struct::<NormalTestStruct>()?)),
                    _ => decoder.skip_field()?,
                }
            }

            Ok(Self {
                p0: p0.ok_or(RocketPackDecoderError::Other("missing field: p0"))?,
                p1: p1.ok_or(RocketPackDecoderError::Other("missing field: p1"))?,
                p2: p2.ok_or(RocketPackDecoderError::Other("missing field: p2"))?,
                p3: p3.ok_or(RocketPackDecoderError::Other("missing field: p3"))?,
                p4: p4.ok_or(RocketPackDecoderError::Other("missing field: p4"))?,
                p5: p5.ok_or(RocketPackDecoderError::Other("missing field: p5"))?,
                p6: p6.ok_or(RocketPackDecoderError::Other("missing field: p6"))?,
                p7: p7.ok_or(RocketPackDecoderError::Other("missing field: p7"))?,
                p8: p8.ok_or(RocketPackDecoderError::Other("missing field: p8"))?,
                p9: p9.ok_or(RocketPackDecoderError::Other("missing field: p9"))?,
                p10: p10.ok_or(RocketPackDecoderError::Other("missing field: p10"))?,
                p11: p11.ok_or(RocketPackDecoderError::Other("missing field: p11"))?,
                p12: p12.ok_or(RocketPackDecoderError::Other("missing field: p12"))?,
                p13,
                p14,
                p15,
            })
        }
    }
}

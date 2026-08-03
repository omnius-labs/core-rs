#[path = "../gen/src/example/first/first.rs"]
mod generated;

use std::collections::BTreeMap;

use generated::omnius::demo::v1::*;

use omnius_core_rocketpack::{
    primitive::{Timestamp64, Timestamp96},
    RocketPackStruct,
};

fn main() {
    run_generated_roundtrip_checks();
    println!("generated round-trip checks passed");
}

fn run_generated_roundtrip_checks() {
    assert_roundtrip(&sample_primitive_showcase_1());
    assert_roundtrip(&sample_primitive_showcase_2());
    assert_roundtrip(&PrimitiveShowcase3::First);
    assert_roundtrip(&sample_primitive_showcase_3_second());
    assert_roundtrip(&sample_primitive_showcase_3_third());
    assert_roundtrip(&Status::Success);
    assert_roundtrip(&Status::Failed);
    assert_eq!(MAX_SAMPLE_SIZE, 1_048_576);
}

fn assert_roundtrip<T>(value: &T)
where
    T: RocketPackStruct + PartialEq + std::fmt::Debug,
{
    let bytes = value.export().expect("failed to export sample value");
    let decoded = T::import(bytes.as_slice()).expect("failed to import sample value");
    assert_eq!(&decoded, value);
}

fn sample_primitive_showcase_1() -> PrimitiveShowcase1 {
    PrimitiveShowcase1 {
        bool_field: true,
        u8_field: 7,
        i16_field: -120,
        i32_field: 12_345,
        i64_field: -987_654_321,
        u16_field: 650,
        u32_field: 99_999,
        u64_field: 123_456_789,
        f32_field: 1.5,
        f64_field: 9.25,
        // 制約なし側と制約あり側は、取り違えが golden byte 列に出るよう別の値にしている
        string_field: "free".to_string(),
        bytes_field: vec![0x01, 0x02],
        vec_field_1: vec![9],
        vec_field_2: vec!["gamma".to_string()],
        vec_field_3: vec![vec![0x60, 0x70]],
        map_field_1: BTreeMap::from([(3_u8, "three".to_string())]),
        map_field_2: BTreeMap::from([("z".to_string(), 26_u8)]),
        map_vec_field_1: BTreeMap::new(),
        map_vec_field_2: BTreeMap::new(),
        slice_field: [-4, -1, 0, 8],
        struct_field: SimpleMessage { bool_field: Some(true) },
        string_field_constrained: "roundtrip".to_string(),
        bytes_field_constrained: vec![0xDE, 0xAD, 0xBE, 0xEF],
        vec_field_1_constrained: vec![1, 2, 3, 4],
        vec_field_2_constrained: vec!["alpha".to_string(), "beta".to_string()],
        vec_field_3_constrained: vec![vec![0x10, 0x20], vec![0x30, 0x40, 0x50]],
        map_field_1_constrained: BTreeMap::from([(1_u8, "one".to_string()), (2_u8, "two".to_string())]),
        map_field_2_constrained: BTreeMap::from([("x".to_string(), 24_u8), ("y".to_string(), 25_u8)]),
        map_vec_field_1_constrained: BTreeMap::new(),
        map_vec_field_2_constrained: BTreeMap::new(),
        timestamp_64: Timestamp64::new(1_700_000_000),
        timestamp_96: Timestamp96::new(1_700_000_000, 123_456_789),
    }
}

fn sample_primitive_showcase_2() -> PrimitiveShowcase2 {
    PrimitiveShowcase2 {
        bool_field: Some(false),
        u8_field: Some(8),
        i16_field: None,
        i32_field: Some(-2_048),
        i64_field: Some(123_456_789),
        u16_field: Some(512),
        u32_field: None,
        u64_field: Some(7_777),
        f32_field: Some(3.25),
        f64_field: None,
        string_field: Some("free".to_string()),
        bytes_field: Some(vec![0x07]),
        vec_field_1: Some(vec![21]),
        vec_field_2: Some(vec!["up".to_string()]),
        vec_field_3: None,
        map_field_1: Some(BTreeMap::from([(4_u8, "four".to_string())])),
        map_field_2: None,
        map_vec_field_1: Some(BTreeMap::new()),
        map_vec_field_2: Some(BTreeMap::new()),
        struct_field: Some(SimpleMessage { bool_field: Some(false) }),
        string_field_constrained: Some("optional".to_string()),
        bytes_field_constrained: Some(vec![0x01, 0x03, 0x05]),
        vec_field_1_constrained: Some(vec![5, 8, 13]),
        vec_field_2_constrained: Some(vec!["left".to_string(), "right".to_string()]),
        vec_field_3_constrained: None,
        map_field_1_constrained: Some(BTreeMap::from([(9_u8, "nine".to_string())])),
        map_field_2_constrained: None,
        map_vec_field_1_constrained: Some(BTreeMap::new()),
        map_vec_field_2_constrained: Some(BTreeMap::new()),
        timestamp_64: Some(Timestamp64::new(-1)),
        timestamp_96: Some(Timestamp96::new(-1, 999_999_999)),
    }
}

fn sample_primitive_showcase_3_second() -> PrimitiveShowcase3 {
    PrimitiveShowcase3::Second {
        entity: "free-worker".to_string(),
        entity_constrained: "worker".to_string(),
        payload: vec![vec![0xAA, 0xBB], vec![0x10, 0x20, 0x30]],
    }
}

fn sample_primitive_showcase_3_third() -> PrimitiveShowcase3 {
    PrimitiveShowcase3::Third {
        entity: "free-job".to_string(),
        entity_constrained: "job-42".to_string(),
        status: Status::Success,
        retries: 3,
        struct_field: Some(SimpleMessage { bool_field: Some(true) }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use omnius_core_rocketpack::{RocketPackBytesEncoder, RocketPackDecoderError, RocketPackEncoderError};

    #[test]
    fn generated_roundtrip_checks_pass() {
        run_generated_roundtrip_checks();
    }

    #[test]
    fn nested_map_roundtrip_materializes_key_before_value() {
        let mut value = sample_primitive_showcase_1();
        value.map_vec_field_1 = BTreeMap::from([("key".to_string(), vec![7, 8])]);
        value.map_vec_field_1_constrained = BTreeMap::from([("key".to_string(), vec![7, 8])]);

        assert_roundtrip(&value);
    }

    #[test]
    fn unconstrained_values_accept_lengths_beyond_the_constrained_limits() {
        let mut value = sample_primitive_showcase_1();
        // 制約付き側の上限 (string 32、Vec<u8> 8、Map 4) を大きく超える値
        value.string_field = "x".repeat(10_000);
        value.bytes_field = vec![0xFF; 10_000];
        value.vec_field_1 = vec![1; 1_000];
        value.vec_field_2 = (0..100).map(|index| index.to_string()).collect();
        value.map_field_1 = (0..=255).map(|index| (index, index.to_string())).collect();

        assert!(value.export().is_ok());
        assert_roundtrip(&value);
    }

    #[test]
    fn generated_wire_bytes_remain_stable() {
        const EXPECTED: &[u8] = &[
            0xB8, 0x20, 0x01, 0xF5, 0x02, 0x07, 0x03, 0x38, 0x77, 0x04, 0x19, 0x30, 0x39, 0x05, 0x3A, 0x3A, 0xDE, 0x68, 0xB0, 0x06, 0x19, 0x02, 0x8A, 0x07, 0x1A, 0x00, 0x01, 0x86, 0x9F,
            0x08, 0x1A, 0x07, 0x5B, 0xCD, 0x15, 0x0A, 0xFA, 0x3F, 0xC0, 0x00, 0x00, 0x0B, 0xFB, 0x40, 0x22, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x64, 0x66, 0x72, 0x65, 0x65, 0x0D,
            0x42, 0x01, 0x02, 0x0E, 0x81, 0x09, 0x0F, 0x81, 0x65, 0x67, 0x61, 0x6D, 0x6D, 0x61, 0x10, 0x81, 0x42, 0x60, 0x70, 0x11, 0xA1, 0x03, 0x65, 0x74, 0x68, 0x72, 0x65, 0x65, 0x12,
            0xA1, 0x61, 0x7A, 0x18, 0x1A, 0x13, 0xA0, 0x14, 0xA0, 0x15, 0x84, 0x23, 0x20, 0x00, 0x08, 0x16, 0xA1, 0x01, 0xF5, 0x17, 0x69, 0x72, 0x6F, 0x75, 0x6E, 0x64, 0x74, 0x72, 0x69,
            0x70, 0x18, 0x18, 0x44, 0xDE, 0xAD, 0xBE, 0xEF, 0x18, 0x19, 0x84, 0x01, 0x02, 0x03, 0x04, 0x18, 0x1A, 0x82, 0x65, 0x61, 0x6C, 0x70, 0x68, 0x61, 0x64, 0x62, 0x65, 0x74, 0x61,
            0x18, 0x1B, 0x82, 0x42, 0x10, 0x20, 0x43, 0x30, 0x40, 0x50, 0x18, 0x1C, 0xA2, 0x01, 0x63, 0x6F, 0x6E, 0x65, 0x02, 0x63, 0x74, 0x77, 0x6F, 0x18, 0x1D, 0xA2, 0x61, 0x78, 0x18,
            0x18, 0x61, 0x79, 0x18, 0x19, 0x18, 0x1E, 0xA0, 0x18, 0x1F, 0xA0, 0x18, 0x20, 0x1A, 0x65, 0x53, 0xF1, 0x00, 0x18, 0x21, 0xA2, 0x00, 0x1A, 0x65, 0x53, 0xF1, 0x00, 0x01, 0x1A,
            0x07, 0x5B, 0xCD, 0x15,
        ];

        assert_eq!(sample_primitive_showcase_1().export().unwrap(), EXPECTED);
    }

    #[test]
    fn bounded_values_accept_their_minimum_and_maximum() {
        let mut minimum = sample_primitive_showcase_1();
        minimum.string_field_constrained = "x".to_string();
        minimum.bytes_field_constrained = vec![1];
        minimum.vec_field_1_constrained = vec![1];
        minimum.vec_field_3_constrained = vec![vec![1]];
        minimum.map_field_1_constrained = BTreeMap::from([(1, "x".to_string())]);
        assert!(minimum.export().is_ok());

        let mut maximum = sample_primitive_showcase_1();
        maximum.string_field_constrained = "x".repeat(32);
        maximum.vec_field_1_constrained = vec![1; 8];
        maximum.vec_field_3_constrained = vec![vec![1; 8]; 4];
        maximum.map_field_1_constrained = BTreeMap::from([(1, "a".repeat(16)), (2, "b".repeat(16)), (3, "c".repeat(16)), (4, "d".repeat(16))]);
        assert!(maximum.export().is_ok());
    }

    #[test]
    fn encode_rejects_invalid_lengths_without_mutating_the_output() {
        let mut value = sample_primitive_showcase_1();
        value.string_field_constrained = String::new();
        let mut output = vec![0xAA, 0xBB];
        let expected_output = output.clone();
        let error = {
            let mut encoder = RocketPackBytesEncoder::new(&mut output);
            PrimitiveShowcase1::pack(&mut encoder, &value).unwrap_err()
        };

        assert!(matches!(
            error,
            RocketPackEncoderError::LengthOutOfRange {
                context: "PrimitiveShowcase1.string_field_constrained",
                min: 1,
                max: 32,
                actual: 0
            }
        ));
        assert_eq!(output, expected_output);
    }

    #[test]
    fn nested_named_validation_precedes_parent_output() {
        let value = NestedParent {
            child: NestedChild { label: String::new() },
        };
        let mut output = vec![0xAA, 0xBB];
        let expected_output = output.clone();
        let error = {
            let mut encoder = RocketPackBytesEncoder::new(&mut output);
            NestedParent::pack(&mut encoder, &value).unwrap_err()
        };

        assert!(matches!(
            error,
            RocketPackEncoderError::LengthOutOfRange {
                context: "NestedChild.label",
                min: 1,
                max: 4,
                actual: 0
            }
        ));
        assert_eq!(output, expected_output);
    }

    #[test]
    fn decode_rejects_lengths_at_each_prefix_position() {
        // tag 24 以上は 0x18 を前置する 2 byte で符号化される
        assert_length_error(&[0xA1, 0x17, 0x60], "PrimitiveShowcase1.string_field_constrained", 0, 2);
        assert_length_error(&[0xA1, 0x17, 0x78, 33], "PrimitiveShowcase1.string_field_constrained", 33, 2);
        assert_length_error(&[0xA1, 0x18, 0x18, 0x40], "PrimitiveShowcase1.bytes_field_constrained", 0, 3);
        assert_length_error(&[0xA1, 0x18, 0x1B, 0x81, 0x49], "PrimitiveShowcase1.vec_field_3_constrained[]", 9, 4);
        assert_length_error(&[0xA1, 0x18, 0x19, 0x80], "PrimitiveShowcase1.vec_field_1_constrained", 0, 3);
        assert_length_error(&[0xA1, 0x18, 0x1C, 0xA0], "PrimitiveShowcase1.map_field_1_constrained", 0, 3);

        // 上限超過を検査するには、宣言した個数ぶんの後続 byte が必要になる。
        // 足りない場合は buffer 長の検査が先に UnexpectedEof を返す
        assert_length_error(&[&[0xA1, 0x18, 0x19, 0x89][..], &[0x00; 9][..]].concat(), "PrimitiveShowcase1.vec_field_1_constrained", 9, 3);
        assert_length_error(&[&[0xA1, 0x18, 0x1C, 0xA5][..], &[0x00; 10][..]].concat(), "PrimitiveShowcase1.map_field_1_constrained", 5, 3);
    }

    #[test]
    fn decode_rejects_counts_larger_than_the_remaining_buffer() {
        // 9 byte で u64::MAX 個の要素を宣言しても、確保する前に弾かれる
        let bytes = [&[0xA1, 0x18, 0x19, 0x9B][..], &u64::MAX.to_be_bytes()[..]].concat();

        assert!(matches!(PrimitiveShowcase1::import(&bytes), Err(RocketPackDecoderError::UnexpectedEof)));
    }

    fn assert_length_error(bytes: &[u8], context: &'static str, actual: u64, position: usize) {
        assert!(matches!(
            PrimitiveShowcase1::import(bytes),
            Err(RocketPackDecoderError::LengthOutOfRange {
                context: actual_context,
                min: _,
                max: _,
                actual: actual_length,
                position: actual_position,
            }) if actual_context == context && actual_length == actual && actual_position == position
        ));
    }
}

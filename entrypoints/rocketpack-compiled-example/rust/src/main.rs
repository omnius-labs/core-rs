#[path = "../gen/src/example/first/first.rs"]
mod generated;

use std::collections::BTreeMap;

use generated::omnius::demo::v1::*;
use omnius_core_rocketpack::RocketPackStruct;

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
        string_field: "roundtrip".to_string(),
        bytes_field: vec![0xDE, 0xAD, 0xBE, 0xEF],
        vec_field_1: vec![1, 2, 3, 4],
        vec_field_2: vec!["alpha".to_string(), "beta".to_string()],
        vec_field_3: vec![vec![0x10, 0x20], vec![0x30, 0x40, 0x50]],
        map_field_1: BTreeMap::from([(1_u8, "one".to_string()), (2_u8, "two".to_string())]),
        map_field_2: BTreeMap::from([("x".to_string(), 24_u8), ("y".to_string(), 25_u8)]),
        map_vec_field_1: BTreeMap::new(),
        map_vec_field_2: BTreeMap::new(),
        slice_field: [-4, -1, 0, 8],
        struct_field: SimpleMessage { bool_field: Some(true) },
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
        string_field: Some("optional".to_string()),
        bytes_field: Some(vec![0x01, 0x03, 0x05]),
        vec_field_1: Some(vec![5, 8, 13]),
        vec_field_2: Some(vec!["left".to_string(), "right".to_string()]),
        vec_field_3: None,
        map_field_1: Some(BTreeMap::from([(9_u8, "nine".to_string())])),
        map_field_2: None,
        map_vec_field_1: Some(BTreeMap::new()),
        map_vec_field_2: Some(BTreeMap::new()),
        struct_field: Some(SimpleMessage { bool_field: Some(false) }),
    }
}

fn sample_primitive_showcase_3_second() -> PrimitiveShowcase3 {
    PrimitiveShowcase3::Second {
        entity: "worker".to_string(),
        payload: vec![vec![0xAA, 0xBB], vec![0x10, 0x20, 0x30]],
    }
}

fn sample_primitive_showcase_3_third() -> PrimitiveShowcase3 {
    PrimitiveShowcase3::Third {
        entity: "job-42".to_string(),
        status: Status::Success,
        retries: 3,
        struct_field: Some(SimpleMessage { bool_field: Some(true) }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_roundtrip_checks_pass() {
        run_generated_roundtrip_checks();
    }
}

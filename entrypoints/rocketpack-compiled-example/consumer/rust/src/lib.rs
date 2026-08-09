#[path = "../gen/src/rocketpack.rs"]
pub mod rocketpack;

#[cfg(test)]
mod tests {
    use omnius_core_rocketpack::RocketPackStruct;
    use provider_alias::rocketpack::omnius::provider::v1::UserId;

    use crate::rocketpack::omnius::consumer::v1::UserEnvelope;

    #[test]
    fn external_type_round_trip() {
        let first = UserId { value: "first".to_string() };
        let value = UserEnvelope {
            user_id: first.clone(),
            history: vec![first, UserId { value: "second".to_string() }],
        };

        let bytes = value.export().expect("failed to export external envelope");
        let decoded = UserEnvelope::import(&bytes).expect("failed to import external envelope");
        assert_eq!(decoded, value);
    }
}

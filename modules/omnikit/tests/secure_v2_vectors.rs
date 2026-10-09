#[cfg(test)]
mod tests {
    use omnius_core_omnikit::generated::omni_secure::{AuthType, V2AuthMessage, V2FinishedMessage, V2ProfileMessage, V2Role};
    use omnius_core_rocketpack::RocketPackStruct;
    use serde_json::Value;
    use testresult::TestResult;

    fn fixtures() -> Value {
        serde_json::from_str(include_str!("fixtures/secure_v2_vectors.json")).unwrap()
    }

    fn bytes(value: &Value) -> Vec<u8> {
        hex::decode(value.as_str().unwrap()).unwrap()
    }

    fn payload(value: &Value, profile: bool) -> Vec<u8> {
        let wire = bytes(value);
        let offset = if profile {
            assert_eq!(&wire[..8], b"OMNISC2\0");
            8
        } else {
            0
        };
        let length = u32::from_le_bytes(wire[offset..offset + 4].try_into().unwrap()) as usize;
        assert_eq!(length, wire.len() - offset - 4);
        wire[offset + 4..].to_vec()
    }

    #[test]
    fn generated_handshake_codecs_match_independent_fixtures() -> TestResult {
        for case in fixtures()["cases"].as_array().unwrap() {
            for role in ["i", "r"] {
                let wire = &case["handshake_wire"];
                let profile_bytes = payload(&wire[format!("profile_{role}_hex")], true);
                let profile = V2ProfileMessage::import(&profile_bytes)?;
                assert_eq!(profile.export()?, profile_bytes);
                assert_eq!(profile.version, 2);
                assert_eq!(profile.role, if role == "i" { V2Role::Connected } else { V2Role::Accepted });
                assert_eq!(profile.auth_type, if case["mode"] == "Mutual" { AuthType::Sign } else { AuthType::None });
                assert_eq!(profile.context, bytes(&case["inputs"]["context_hex"]));

                let auth_bytes = payload(&wire[format!("auth_{role}_hex")], false);
                let auth = V2AuthMessage::import(&auth_bytes)?;
                assert_eq!(auth.export()?, auth_bytes);
                if let Some(cert) = auth.cert {
                    assert_eq!(cert.name, profile.name);
                    assert_eq!(cert.public_key, profile.public_key);
                } else {
                    assert_eq!(case["mode"], "Anonymous");
                    assert!(profile.name.is_empty());
                    assert!(profile.public_key.is_empty());
                }

                let finished_bytes = payload(&wire[format!("finished_{role}_hex")], false);
                let finished = V2FinishedMessage::import(&finished_bytes)?;
                assert_eq!(finished.export()?, finished_bytes);
                assert_eq!(finished.verify_data, bytes(&case["crypto"][format!("verify_{role}_hex")]));
            }
        }
        Ok(())
    }

    #[test]
    fn signatures_reject_legacy_nonce_role_reflection_and_changed_transcript() -> TestResult {
        let data = fixtures();
        let case = &data["cases"][0];
        assert_eq!(case["mode"], "Mutual");
        for (index, role) in ["i", "r"].into_iter().enumerate() {
            let auth = V2AuthMessage::import(&payload(&case["handshake_wire"][format!("auth_{role}_hex")], false))?;
            let cert = auth.cert.unwrap();
            let preimage = bytes(&case["crypto"][format!("{role}_signature_preimage_hex")]);
            cert.verify(&preimage)?;

            let mut reflected = preimage.clone();
            let role_offset = reflected.len() - 33;
            reflected[role_offset] = if reflected[role_offset] == 1 { 2 } else { 1 };
            assert!(cert.verify(&reflected).is_err());
            let mut changed = preimage.clone();
            *changed.last_mut().unwrap() ^= 1;
            assert!(cert.verify(&changed).is_err());

            let mut legacy = cert;
            legacy.value = bytes(&case["legacy_signatures_hex"][index]);
            legacy.verify(&bytes(&case["crypto"]["t0_hex"]))?;
            assert!(legacy.verify(&preimage).is_err());
        }
        Ok(())
    }
}

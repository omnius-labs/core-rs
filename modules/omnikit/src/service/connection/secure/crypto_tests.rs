#[cfg(test)]
mod tests {
    use serde_json::Value;
    use testresult::TestResult;
    use zeroize::{ZeroizeOnDrop, Zeroizing};

    use super::super::{
        kdf::V2Kdf,
        record::{RecordHeader, RecordKind, TrafficState},
        transcript::HandshakeTranscript,
    };
    use crate::{
        generated::omni_secure::{V2AuthMessage, V2ProfileMessage},
        prelude::RocketPackStruct,
    };

    fn bytes(value: &Value) -> Vec<u8> {
        hex::decode(value.as_str().unwrap()).unwrap()
    }
    fn fixtures() -> Value {
        serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/secure_v2_vectors.json"))).unwrap()
    }

    #[test]
    fn production_transcript_and_finished_match_independent_fixtures() -> TestResult {
        for c in fixtures()["cases"].as_array().unwrap() {
            let pi = V2ProfileMessage::import(&bytes(&c["handshake_wire"]["profile_i_hex"])[12..])?;
            let pr = V2ProfileMessage::import(&bytes(&c["handshake_wire"]["profile_r_hex"])[12..])?;
            let ai = V2AuthMessage::import(&bytes(&c["handshake_wire"]["auth_i_hex"])[4..])?;
            let ar = V2AuthMessage::import(&bytes(&c["handshake_wire"]["auth_r_hex"])[4..])?;
            assert_eq!(HandshakeTranscript::profile(&pi), bytes(&c["canonical"]["profile_i_hex"]));
            assert_eq!(HandshakeTranscript::profile(&pr), bytes(&c["canonical"]["profile_r_hex"]));
            assert_eq!(HandshakeTranscript::auth(&ai), bytes(&c["canonical"]["auth_i_hex"]));
            assert_eq!(HandshakeTranscript::auth(&ar), bytes(&c["canonical"]["auth_r_hex"]));
            let t0 = HandshakeTranscript::hello(&pi, &pr);
            assert_eq!(t0.as_slice(), bytes(&c["crypto"]["t0_hex"]));
            let t1 = HandshakeTranscript::authenticated(&t0, &ai, &ar);
            assert_eq!(t1.as_slice(), bytes(&c["crypto"]["t1_hex"]));
            let z = bytes(&c["crypto"]["shared_secret_hex"]);
            let prk = V2Kdf::hmac(&t0, &[&z]);
            assert_eq!(prk.as_slice(), bytes(&c["crypto"]["prk_hex"]));
            let fi = V2Kdf::expand::<32>(prk.as_slice(), &[b"omnius.secure.v2/finished/initiator\0", &t1]);
            let fr = V2Kdf::expand::<32>(prk.as_slice(), &[b"omnius.secure.v2/finished/responder\0", &t1]);
            assert_eq!(fi.as_slice(), bytes(&c["crypto"]["i_finished_key_hex"]));
            assert_eq!(fr.as_slice(), bytes(&c["crypto"]["r_finished_key_hex"]));
            let vi = V2Kdf::hmac(fi.as_slice(), &[b"omnius.secure.v2/verify/initiator\0", &t1]);
            let vr = V2Kdf::hmac(fr.as_slice(), &[b"omnius.secure.v2/verify/responder\0", &t1, vi.as_slice()]);
            assert_eq!(vi.as_slice(), bytes(&c["crypto"]["verify_i_hex"]));
            assert_eq!(vr.as_slice(), bytes(&c["crypto"]["verify_r_hex"]));
            let t2 = HandshakeTranscript::session(&t1, vi.as_slice(), vr.as_slice());
            assert_eq!(t2.as_slice(), bytes(&c["crypto"]["t2_hex"]));
        }
        Ok(())
    }

    #[test]
    fn production_records_and_rekey_match_both_direction_fixtures() -> TestResult {
        for c in fixtures()["cases"].as_array().unwrap() {
            for (role, direction) in [("i", 1), ("r", 2)] {
                let secret: [u8; 32] = bytes(&c["crypto"][format!("{role}_secret_0_hex")]).try_into().unwrap();
                let t2 = bytes(&c["crypto"]["t2_hex"]).try_into().unwrap();
                let mut sender = TrafficState::new(Zeroizing::new(secret), t2, direction);
                let mut receiver = TrafficState::new(Zeroizing::new(secret), t2, direction);
                for (suffix, kind) in [
                    ("data_0", RecordKind::Data),
                    ("update_0", RecordKind::KeyUpdate),
                    ("data_1", RecordKind::Data),
                    ("close_1", RecordKind::Close),
                ] {
                    let record = &c["records"][format!("{role}_{suffix}")];
                    let plaintext = bytes(&record["plaintext_hex"]);
                    let wire = sender.encode(kind, &plaintext)?;
                    assert_eq!(wire, bytes(&record["wire_hex"]));
                    let header = RecordHeader::parse(wire[..21].try_into().unwrap())?;
                    assert_eq!(receiver.decode(&header, &wire[21..])?.as_slice(), plaintext);
                    if kind == RecordKind::KeyUpdate {
                        sender.advance()?;
                    }
                }
            }
        }
        Ok(())
    }

    #[test]
    fn cryptographic_hash_and_aes_schedule_enable_drop_zeroization() {
        fn check<T: ZeroizeOnDrop>() {}
        check::<sha3::Sha3_256>();
        check::<aes::Aes256>();
        check::<x25519_dalek::StaticSecret>();
        check::<ed25519_dalek::SigningKey>();
    }

    #[test]
    fn ghash_held_state_participates_in_normal_drop() {
        // finalize しない保持状態でも Drop が走る構成を検査する。
        // 消去内容は固定した POLYVAL 0.7.3 と zeroize feature の source audit が根拠。
        assert!(std::mem::needs_drop::<polyval::Polyval>());
        assert!(std::mem::needs_drop::<ghash::GHash>());
        assert!(std::mem::needs_drop::<aes_gcm::Aes256Gcm>());
    }
}

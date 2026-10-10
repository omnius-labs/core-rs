use std::{sync::Arc, time::Duration};

use chrono::Utc;
use enumflags2::make_bitflags;
use hkdf::SimpleHkdf;
use hmac::{KeyInit as _, Mac as _, SimpleHmac};
use parking_lot::Mutex;
use rand::RngExt;
use sha3::{Digest, Sha3_256};
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _, ReadHalf, WriteHalf};

use omnius_core_base::clock::Clock;

use crate::{
    generated::{
        omni_agreement::{OmniAgreement, OmniAgreementAlgorithmType, OmniAgreementPublicKey},
        omni_secure::KeyConfirmationMessage,
        omni_sign::{OmniCert, OmniSigner},
    },
    model::omni_secure::{CipherAlgorithmType, HashAlgorithmType, KeyDerivationAlgorithmType, KeyExchangeAlgorithmType},
    prelude::*,
};

use super::{AuthType, OmniSecureStreamType, ProfileMessage};

pub(crate) struct Authenticator<T>
where
    T: AsyncRead + AsyncWrite + Send + 'static,
{
    role: u8,
    reader: ReadHalf<T>,
    writer: WriteHalf<T>,
    max_frame_length: usize,
    signer: Option<OmniSigner>,
    require_peer_signature: bool,
    max_clock_skew: Duration,
    clock: Arc<dyn Clock<Utc> + Send + Sync>,
    rng: Arc<Mutex<dyn rand::Rng + Send + Sync>>,
}

pub(crate) struct AuthResult {
    pub peer_cert: Option<OmniCert>,
    pub enc_key: Vec<u8>,
    pub enc_nonce: Vec<u8>,
    pub dec_key: Vec<u8>,
    pub dec_nonce: Vec<u8>,
}

impl<T> Authenticator<T>
where
    T: AsyncRead + AsyncWrite + Send + 'static,
{
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        typ: OmniSecureStreamType,
        reader: ReadHalf<T>,
        writer: WriteHalf<T>,
        max_frame_length: usize,
        signer: Option<OmniSigner>,
        require_peer_signature: bool,
        max_clock_skew: Duration,
        clock: Arc<dyn Clock<Utc> + Send + Sync>,
        rng: Arc<Mutex<dyn rand::Rng + Send + Sync>>,
    ) -> Result<Self> {
        if max_clock_skew.subsec_nanos() != 0 {
            return Err(Error::new(ErrorKind::InvalidFormat).with_message("clock tolerance must be whole seconds"));
        }
        let role = match typ {
            OmniSecureStreamType::Connected => 0x01,
            OmniSecureStreamType::Accepted => 0x02,
        };
        Ok(Self {
            role,
            reader,
            writer,
            max_frame_length,
            signer,
            require_peer_signature,
            max_clock_skew,
            clock,
            rng,
        })
    }

    pub fn into_inner(self) -> (ReadHalf<T>, WriteHalf<T>) {
        (self.reader, self.writer)
    }

    pub async fn auth(&mut self) -> Result<AuthResult> {
        let my_profile = ProfileMessage::new(
            self.role,
            self.rng.lock().random::<[u8; 32]>().to_vec(),
            if self.signer.is_some() { AuthType::Sign } else { AuthType::None },
            make_bitflags!(KeyExchangeAlgorithmType::X25519),
            make_bitflags!(KeyDerivationAlgorithmType::HKDF),
            make_bitflags!(CipherAlgorithmType::AES_256_GCM),
            make_bitflags!(HashAlgorithmType::SHA3_256),
        );
        if self.role == 0x01 {
            self.send(&my_profile).await?;
        }
        let other_profile: ProfileMessage = self.recv().await?;
        self.validate_profile(&other_profile)?;
        if self.role == 0x02 {
            self.send(&my_profile).await?;
        }

        let my_agreement = OmniAgreement::new(OmniAgreementAlgorithmType::X25519, self.clock.now())?;
        let my_public_key = my_agreement.gen_agreement_public_key();
        if self.role == 0x01 {
            self.send(&my_public_key).await?;
        }
        let other_public_key: OmniAgreementPublicKey = self.recv().await?;
        self.validate_agreement(&other_public_key)?;
        if self.role == 0x02 {
            self.send(&my_public_key).await?;
        }

        let (connected_profile, accepted_profile, connected_key, accepted_key) = if self.role == 0x01 {
            (&my_profile, &other_profile, &my_public_key, &other_public_key)
        } else {
            (&other_profile, &my_profile, &other_public_key, &my_public_key)
        };
        let transcript = Self::transcript(connected_profile, accepted_profile, connected_key, accepted_key);
        let secret = OmniAgreement::gen_secret(&my_agreement.gen_agreement_private_key(), &other_public_key)?;
        let my_hash = Self::signature_hash(self.role, &transcript);
        let my_cert = self.signer.as_ref().map(|signer| signer.sign(&my_hash)).transpose()?;
        if self.role == 0x01
            && let Some(cert) = &my_cert
        {
            self.send(cert).await?;
        }
        let other_cert = if other_profile.auth_type == AuthType::Sign {
            let cert: OmniCert = self.recv().await?;
            cert.verify(&Self::signature_hash(other_profile.role, &transcript))?;
            Some(cert)
        } else {
            None
        };
        if self.role == 0x02
            && let Some(cert) = &my_cert
        {
            self.send(cert).await?;
        }

        let mut salt = connected_profile.session_id.clone();
        salt.extend_from_slice(&accepted_profile.session_id);
        let mut info = b"OmniSecureStream/V1/keys\0".to_vec();
        info.extend_from_slice(&Sha3_256::digest(&transcript));
        let mut okm = [0u8; 152];
        SimpleHkdf::<Sha3_256>::new(Some(&salt), &secret)
            .expand(&info, &mut okm)
            .map_err(|_| Error::new(ErrorKind::InvalidFormat).with_message("failed to expand key"))?;

        let (connected_cert, accepted_cert, enc_offset, dec_offset, my_mac_key, other_mac_key) = if self.role == 0x01 {
            (my_cert.as_ref(), other_cert.as_ref(), 0, 44, &okm[88..120], &okm[120..152])
        } else {
            (other_cert.as_ref(), my_cert.as_ref(), 44, 0, &okm[120..152], &okm[88..120])
        };
        let my_mac = Self::confirmation_mac(my_mac_key, self.role, &transcript, connected_cert, accepted_cert)?;
        let other_mac = Self::confirmation_mac(other_mac_key, other_profile.role, &transcript, connected_cert, accepted_cert)?;
        let confirmation = KeyConfirmationMessage {
            mac: my_mac.finalize().into_bytes().to_vec(),
        };
        if self.role == 0x01 {
            self.send(&confirmation).await?;
        }
        let other_confirmation: KeyConfirmationMessage = self.recv().await?;
        other_mac
            .verify_slice(&other_confirmation.mac)
            .map_err(|e| Error::from_error(e, ErrorKind::InvalidFormat).with_message("key confirmation failed"))?;
        if self.role == 0x02 {
            self.send(&confirmation).await?;
        }

        Ok(AuthResult {
            peer_cert: other_cert,
            enc_key: okm[enc_offset..enc_offset + 32].to_vec(),
            enc_nonce: okm[enc_offset + 32..enc_offset + 44].to_vec(),
            dec_key: okm[dec_offset..dec_offset + 32].to_vec(),
            dec_nonce: okm[dec_offset + 32..dec_offset + 44].to_vec(),
        })
    }

    async fn send<M: RocketPackStruct>(&mut self, message: &M) -> Result<()> {
        let payload = message
            .export()
            .map_err(|e| Error::from_error(e, ErrorKind::InvalidFormat).with_message("invalid handshake message"))?;
        let length = u32::try_from(payload.len()).map_err(|_| Error::new(ErrorKind::InvalidFormat).with_message("handshake frame too long"))?;
        if payload.len() > self.max_frame_length {
            return Err(Error::new(ErrorKind::InvalidFormat).with_message("handshake frame too long"));
        }
        self.writer.write_all(&length.to_le_bytes()).await?;
        self.writer.write_all(&payload).await?;
        self.writer.flush().await?;
        Ok(())
    }

    async fn recv<M: RocketPackStruct>(&mut self) -> Result<M> {
        // Read exactly this frame: the next bytes may already be encrypted data.
        let mut header = [0u8; 4];
        self.reader.read_exact(&mut header).await?;
        let length = u32::from_le_bytes(header) as usize;
        if length > self.max_frame_length {
            return Err(Error::new(ErrorKind::InvalidFormat).with_message("handshake frame too long"));
        }
        let mut payload = vec![0u8; length];
        self.reader.read_exact(&mut payload).await?;
        M::import(&payload).map_err(|e| Error::from_error(e, ErrorKind::InvalidFormat).with_message("invalid handshake message"))
    }

    fn validate_profile(&self, profile: &ProfileMessage) -> Result<()> {
        if profile.role != 3 - self.role {
            return Err(Error::new(ErrorKind::InvalidFormat).with_message("invalid peer role"));
        }
        if profile.session_id.len() != 32 {
            return Err(Error::new(ErrorKind::InvalidFormat).with_message("invalid session ID"));
        }
        if self.require_peer_signature && profile.auth_type != AuthType::Sign {
            return Err(Error::new(ErrorKind::InvalidFormat).with_message("peer signature required"));
        }
        if [
            profile.key_exchange_algorithm_type_flags,
            profile.key_derivation_algorithm_type_flags,
            profile.cipher_algorithm_type_flags,
            profile.hash_algorithm_type_flags,
        ] != [1; 4]
        {
            return Err(Error::new(ErrorKind::UnsupportedType).with_message("handshake algorithm"));
        }
        Ok(())
    }

    fn validate_agreement(&self, key: &OmniAgreementPublicKey) -> Result<()> {
        if key.algorithm_type != OmniAgreementAlgorithmType::X25519 {
            return Err(Error::new(ErrorKind::UnsupportedType).with_message("key exchange algorithm"));
        }
        if key.public_key.len() != 32 {
            return Err(Error::new(ErrorKind::InvalidFormat).with_message("invalid agreement key"));
        }
        let difference = (i128::from(self.clock.now().timestamp()) - i128::from(key.created_time.seconds)).abs();
        if difference > i128::from(self.max_clock_skew.as_secs()) {
            return Err(Error::new(ErrorKind::InvalidFormat).with_message("agreement key outside time tolerance"));
        }
        Ok(())
    }

    fn transcript(connected_profile: &ProfileMessage, accepted_profile: &ProfileMessage, connected_key: &OmniAgreementPublicKey, accepted_key: &OmniAgreementPublicKey) -> Vec<u8> {
        let mut transcript = b"OmniSecureStream/V1/transcript\0".to_vec();
        for (profile, key) in [(connected_profile, connected_key), (accepted_profile, accepted_key)] {
            transcript.push(profile.role);
            transcript.extend_from_slice(&profile.session_id);
            transcript.extend_from_slice(&profile.auth_type.to_u32().to_le_bytes());
            transcript.extend_from_slice(&profile.key_exchange_algorithm_type_flags.to_le_bytes());
            transcript.extend_from_slice(&profile.key_derivation_algorithm_type_flags.to_le_bytes());
            transcript.extend_from_slice(&profile.cipher_algorithm_type_flags.to_le_bytes());
            transcript.extend_from_slice(&profile.hash_algorithm_type_flags.to_le_bytes());
            transcript.extend_from_slice(&key.created_time.seconds.to_be_bytes());
            transcript.extend_from_slice(&key.algorithm_type.to_u32().to_le_bytes());
            transcript.extend_from_slice(&key.public_key);
        }
        transcript
    }

    fn signature_hash(role: u8, transcript: &[u8]) -> Vec<u8> {
        let mut hash = Sha3_256::new();
        hash.update(b"OmniSecureStream/V1/signature\0");
        hash.update([role]);
        hash.update(transcript);
        hash.finalize().to_vec()
    }

    fn confirmation_mac(key: &[u8], role: u8, transcript: &[u8], connected_cert: Option<&OmniCert>, accepted_cert: Option<&OmniCert>) -> Result<SimpleHmac<Sha3_256>> {
        let mut mac = SimpleHmac::<Sha3_256>::new_from_slice(key).map_err(|e| Error::from_error(e, ErrorKind::InvalidFormat).with_message("invalid MAC key"))?;
        mac.update(b"OmniSecureStream/V1/confirmation\0");
        mac.update(&[role]);
        mac.update(transcript);
        for cert in [connected_cert, accepted_cert] {
            if let Some(cert) = cert {
                mac.update(&[0x01]);
                // Signing and verification accept only the type with RPF tag 2.
                mac.update(&2_u32.to_le_bytes());
                mac.update(&(cert.public_key.len() as u32).to_le_bytes());
                mac.update(&cert.public_key);
            } else {
                mac.update(&[0x00]);
            }
        }
        Ok(mac)
    }
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use omnius_core_base::clock::FakeClockUtc;
    use omnius_core_rocketpack::primitive::Timestamp64;
    use rand::{SeedableRng as _, rngs::ChaCha20Rng};
    use testresult::TestResult;
    use tokio::{io::DuplexStream, time::timeout};

    use crate::{generated::omni_sign::OmniSignType, service::connection::secure::OmniSecureStream};

    use super::*;

    type TestAuth = Authenticator<DuplexStream>;
    const NOW: i64 = 1_700_000_000;

    fn clock(seconds: i64) -> Arc<FakeClockUtc> {
        Arc::new(FakeClockUtc::new(DateTime::from_timestamp(seconds, 0).unwrap()))
    }

    fn signer(name: &str) -> OmniSigner {
        OmniSigner::new(OmniSignType::Ed25519_Sha3_256_Base64Url, name).unwrap()
    }

    async fn authenticate(stream: DuplexStream, typ: OmniSecureStreamType, signer: Option<OmniSigner>, required: bool, seconds: i64) -> Result<AuthResult> {
        let (reader, writer) = tokio::io::split(stream);
        let mut auth = TestAuth::new(
            typ,
            reader,
            writer,
            16384,
            signer,
            required,
            Duration::from_secs(300),
            clock(seconds),
            Arc::new(Mutex::new(ChaCha20Rng::from_seed([42; 32]))),
        )
        .await?;
        auth.auth().await
    }

    async fn read_frame(stream: &mut DuplexStream) -> Result<Vec<u8>> {
        let length = stream.read_u32_le().await?;
        assert!(length <= 16384);
        let mut payload = vec![0; length as usize];
        stream.read_exact(&mut payload).await?;
        Ok(payload)
    }

    async fn write_frame(stream: &mut DuplexStream, payload: &[u8]) -> Result<()> {
        stream.write_u32_le(payload.len() as u32).await?;
        stream.write_all(payload).await?;
        stream.flush().await?;
        Ok(())
    }

    async fn run_relay(connected_signs: bool, accepted_signs: bool, mut mutate: impl FnMut(u8, usize, Vec<u8>) -> Vec<u8>) -> (Result<AuthResult>, Result<AuthResult>) {
        // A tiny transport also exercises the specified send/receive ordering.
        let (connected, mut left) = tokio::io::duplex(32);
        let (accepted, mut right) = tokio::io::duplex(32);
        let relay = async move {
            for stage in 0..4 {
                for role in [0x01, 0x02] {
                    if stage == 2 && !(if role == 0x01 { connected_signs } else { accepted_signs }) {
                        continue;
                    }
                    let (from, to) = if role == 0x01 { (&mut left, &mut right) } else { (&mut right, &mut left) };
                    let payload = read_frame(from).await?;
                    let payload = mutate(role, stage, payload);
                    write_frame(to, &payload).await?;
                }
            }
            Ok::<_, Error>(())
        };
        let (connected, accepted, _) = timeout(Duration::from_secs(5), async {
            tokio::join!(
                authenticate(connected, OmniSecureStreamType::Connected, connected_signs.then(|| signer("connected")), false, NOW),
                authenticate(accepted, OmniSecureStreamType::Accepted, accepted_signs.then(|| signer("accepted")), false, NOW),
                relay
            )
        })
        .await
        .expect("handshake deadlocked");
        (connected, accepted)
    }

    fn assert_rejection<T>(result: Result<T>, message: &str) {
        let error = match result {
            Err(error) => error,
            Ok(_) => panic!("handshake unexpectedly succeeded"),
        };
        assert_eq!(error.kind(), &ErrorKind::InvalidFormat);
        assert_eq!(error.message(), Some(message));
    }

    #[test]
    fn signature_preimage_uses_explicit_rpf_tags_not_wire_export() {
        let connected = ProfileMessage {
            role: 1,
            session_id: vec![7; 32],
            auth_type: AuthType::Sign,
            key_exchange_algorithm_type_flags: 1,
            key_derivation_algorithm_type_flags: 1,
            cipher_algorithm_type_flags: 1,
            hash_algorithm_type_flags: 1,
        };
        let accepted = ProfileMessage {
            role: 2,
            session_id: vec![8; 32],
            auth_type: AuthType::None,
            ..connected.clone()
        };
        let key = OmniAgreementPublicKey {
            algorithm_type: OmniAgreementAlgorithmType::X25519,
            public_key: vec![9; 32],
            created_time: Timestamp64::new(11),
        };
        let other_key = OmniAgreementPublicKey {
            public_key: vec![10; 32],
            created_time: Timestamp64::new(12),
            ..key.clone()
        };
        let actual = TestAuth::transcript(&connected, &accepted, &key, &other_key);
        let mut expected = b"OmniSecureStream/V1/transcript\0".to_vec();
        for (role, session, auth, seconds, public_key) in [(1, 7, 2_u32, 11_i64, 9), (2, 8, 1, 12, 10)] {
            expected.push(role);
            expected.extend_from_slice(&[session; 32]);
            expected.extend_from_slice(&auth.to_le_bytes());
            for _ in 0..4 {
                expected.extend_from_slice(&1_u32.to_le_bytes());
            }
            expected.extend_from_slice(&seconds.to_be_bytes());
            expected.extend_from_slice(&2_u32.to_le_bytes());
            expected.extend_from_slice(&[public_key; 32]);
        }
        assert_eq!(actual, expected);
        let mut preimage = b"OmniSecureStream/V1/signature\0\x01".to_vec();
        preimage.extend_from_slice(&expected);
        assert_eq!(TestAuth::signature_hash(1, &actual), Sha3_256::digest(preimage).to_vec());
        assert_ne!(TestAuth::signature_hash(1, &actual), TestAuth::signature_hash(2, &actual));
    }

    #[tokio::test]
    async fn handshake_rejects_substituted_signature() -> TestResult {
        for (connected_signs, accepted_signs, target_role) in [(true, true, 1), (true, true, 2), (true, false, 1), (false, true, 2)] {
            let attacker = signer("attacker");
            let mut profiles = [None, None];
            let mut keys = [None, None];
            let mut substituted = false;
            let (_, accepted) = run_relay(connected_signs, accepted_signs, |role, stage, payload| {
                let index = usize::from(role - 1);
                match stage {
                    0 => profiles[index] = Some(ProfileMessage::import(&payload).unwrap()),
                    1 => keys[index] = Some(OmniAgreementPublicKey::import(&payload).unwrap()),
                    2 if role == target_role => {
                        let transcript = TestAuth::transcript(
                            profiles[0].as_ref().unwrap(),
                            profiles[1].as_ref().unwrap(),
                            keys[0].as_ref().unwrap(),
                            keys[1].as_ref().unwrap(),
                        );
                        let hash = TestAuth::signature_hash(role, &transcript);
                        let original = OmniCert::import(&payload).unwrap();
                        original.verify(&hash).unwrap();
                        let replacement = attacker.sign(&hash).unwrap();
                        replacement.verify(&hash).unwrap();
                        assert_ne!(original.public_key, replacement.public_key);
                        substituted = true;
                        return replacement.export().unwrap();
                    }
                    _ => {}
                }
                payload
            })
            .await;
            assert!(substituted);
            // Connected sends the first MAC. Accepted detects the differing
            // cert slots even when its own cert was replaced in transit.
            assert_rejection(accepted, "key confirmation failed");
        }
        Ok(())
    }

    #[tokio::test]
    async fn handshake_rejects_replayed_messages() -> TestResult {
        let mut recording = Vec::new();
        let mut old_accepted_profile = None;
        let (connected, accepted) = run_relay(true, true, |role, stage, payload| {
            if role == 1 {
                recording.push(payload.clone());
            }
            if role == 2 && stage == 0 {
                old_accepted_profile = Some(ProfileMessage::import(&payload).unwrap());
            }
            payload
        })
        .await;
        connected?;
        accepted?;
        assert_eq!(recording.len(), 4);
        let (mut replay, accepted) = tokio::io::duplex(32);
        // A new RNG seed gives the recipient a fresh session ID. The clock is
        // unchanged, so this must fail signature verification, not freshness.
        let recipient = async move {
            let (reader, writer) = tokio::io::split(accepted);
            let mut auth = TestAuth::new(
                OmniSecureStreamType::Accepted,
                reader,
                writer,
                16384,
                Some(signer("accepted")),
                true,
                Duration::from_secs(300),
                clock(NOW),
                Arc::new(Mutex::new(ChaCha20Rng::from_seed([43; 32]))),
            )
            .await?;
            auth.auth().await
        };
        let replay = async move {
            write_frame(&mut replay, &recording[0]).await?;
            let profile = ProfileMessage::import(&read_frame(&mut replay).await?)?;
            assert_ne!(profile.session_id, old_accepted_profile.unwrap().session_id);
            write_frame(&mut replay, &recording[1]).await?;
            read_frame(&mut replay).await?;
            write_frame(&mut replay, &recording[2]).await?;
            // The recipient rejects the recorded cert before returning its cert.
            assert!(read_frame(&mut replay).await.is_err());
            Ok::<_, Error>(())
        };
        let (result, replay) = timeout(Duration::from_secs(5), async { tokio::join!(recipient, replay) }).await?;
        replay?;
        assert_rejection(result, "failed to verify");
        Ok(())
    }

    #[tokio::test]
    async fn handshake_rejects_tampered_profile_or_key() -> TestResult {
        for stage_to_change in [0, 1] {
            let replacement_key = OmniAgreement::new(OmniAgreementAlgorithmType::X25519, DateTime::from_timestamp(NOW, 0).unwrap())?.gen_agreement_public_key();
            let mut changed = false;
            let (_, accepted) = run_relay(true, true, |role, stage, payload| {
                if role == 1 && stage == stage_to_change {
                    changed = true;
                    if stage == 0 {
                        let mut profile = ProfileMessage::import(&payload).unwrap();
                        profile.session_id[0] ^= 1;
                        return profile.export().unwrap();
                    }
                    let mut key = OmniAgreementPublicKey::import(&payload).unwrap();
                    key.public_key = replacement_key.public_key.clone();
                    return key.export().unwrap();
                }
                payload
            })
            .await;
            assert!(changed);
            assert_rejection(accepted, "failed to verify");
        }
        Ok(())
    }

    #[tokio::test]
    async fn handshake_rejects_unsigned_peer_when_signature_is_required() -> TestResult {
        for required_on_connected in [true, false] {
            let (connected, accepted) = tokio::io::duplex(32);
            let rng = Arc::new(Mutex::new(ChaCha20Rng::from_seed([42; 32])));
            let (connected, accepted) = timeout(Duration::from_secs(5), async {
                tokio::join!(
                    OmniSecureStream::new(
                        connected,
                        OmniSecureStreamType::Connected,
                        16384,
                        None,
                        required_on_connected,
                        Duration::from_secs(300),
                        clock(NOW),
                        rng.clone()
                    ),
                    OmniSecureStream::new(
                        accepted,
                        OmniSecureStreamType::Accepted,
                        16384,
                        None,
                        !required_on_connected,
                        Duration::from_secs(300),
                        clock(NOW),
                        rng.clone()
                    )
                )
            })
            .await?;
            assert_rejection(if required_on_connected { connected } else { accepted }, "peer signature required");
        }
        Ok(())
    }

    #[tokio::test]
    async fn handshake_rejects_agreement_key_outside_time_tolerance() -> TestResult {
        for target_role in [1, 2] {
            for seconds in [NOW - 301, NOW + 301, i64::MIN, i64::MAX] {
                let mut changed = false;
                let (connected, accepted) = run_relay(true, true, |role, stage, payload| {
                    if role == target_role && stage == 1 {
                        let mut key = OmniAgreementPublicKey::import(&payload).unwrap();
                        key.created_time.seconds = seconds;
                        changed = true;
                        return key.export().unwrap();
                    }
                    payload
                })
                .await;
                assert!(changed);
                assert_rejection(if target_role == 1 { accepted } else { connected }, "agreement key outside time tolerance");
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn handshake_accepts_time_tolerance_boundary() -> TestResult {
        for offset in [-300, 0, 300] {
            let (connected, accepted) = tokio::io::duplex(32);
            let (connected, accepted) = timeout(Duration::from_secs(5), async {
                tokio::join!(
                    authenticate(connected, OmniSecureStreamType::Connected, Some(signer("connected")), true, NOW + offset),
                    authenticate(accepted, OmniSecureStreamType::Accepted, Some(signer("accepted")), true, NOW)
                )
            })
            .await?;
            assert!(connected?.peer_cert.is_some());
            assert!(accepted?.peer_cert.is_some());
        }
        Ok(())
    }

    #[tokio::test]
    async fn handshake_exposes_peer_cert_and_preserves_first_encrypted_frame() -> TestResult {
        for (connected_signs, accepted_signs) in [(true, true), (true, false), (false, true), (false, false)] {
            let connected_signer = connected_signs.then(|| signer("connected"));
            let accepted_signer = accepted_signs.then(|| signer("accepted"));
            let connected_public_key = connected_signer.as_ref().map(|signer| signer.sign(b"test").unwrap().public_key);
            let accepted_public_key = accepted_signer.as_ref().map(|signer| signer.sign(b"test").unwrap().public_key);
            let (connected, accepted) = tokio::io::duplex(32);
            let rng = Arc::new(Mutex::new(ChaCha20Rng::from_seed([42; 32])));
            let connected = async {
                let mut stream = OmniSecureStream::new(
                    connected,
                    OmniSecureStreamType::Connected,
                    16384,
                    connected_signer,
                    accepted_signs,
                    Duration::from_secs(300),
                    clock(NOW),
                    rng.clone(),
                )
                .await?;
                assert_eq!(stream.peer_cert().map(|cert| &cert.public_key), accepted_public_key.as_ref());
                assert_eq!(stream.sign_id().map(str::to_owned), stream.peer_cert().map(ToString::to_string));
                let mut message = [0; 5];
                stream.read_exact(&mut message).await?;
                assert_eq!(&message, b"hello");
                stream.write_all(b"reply").await?;
                stream.flush().await?;
                Ok::<_, Error>(())
            };
            let accepted = async {
                let mut stream = OmniSecureStream::new(
                    accepted,
                    OmniSecureStreamType::Accepted,
                    16384,
                    accepted_signer,
                    connected_signs,
                    Duration::from_secs(300),
                    clock(NOW),
                    rng.clone(),
                )
                .await?;
                assert_eq!(stream.peer_cert().map(|cert| &cert.public_key), connected_public_key.as_ref());
                assert_eq!(stream.sign_id().map(str::to_owned), stream.peer_cert().map(ToString::to_string));
                // Write immediately after the final MAC, while Connected may
                // still be consuming that MAC, to detect handshake prefetch.
                stream.write_all(b"hello").await?;
                stream.flush().await?;
                let mut message = [0; 5];
                stream.read_exact(&mut message).await?;
                assert_eq!(&message, b"reply");
                Ok::<_, Error>(())
            };
            let (connected, accepted) = timeout(Duration::from_secs(5), async { tokio::join!(connected, accepted) }).await?;
            connected?;
            accepted?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn handshake_validates_clock_tolerance_argument() -> TestResult {
        let (connected, _accepted) = tokio::io::duplex(32);
        let rng = Arc::new(Mutex::new(ChaCha20Rng::from_seed([42; 32])));
        let result = OmniSecureStream::new(
            connected,
            OmniSecureStreamType::Connected,
            16384,
            None,
            false,
            Duration::from_millis(1),
            clock(NOW),
            rng.clone(),
        )
        .await;
        assert_rejection(result, "clock tolerance must be whole seconds");
        for offset in [-1, 0, 1] {
            let (connected, accepted) = tokio::io::duplex(32);
            let (connected, accepted) = timeout(Duration::from_secs(5), async {
                tokio::join!(
                    OmniSecureStream::new(
                        connected,
                        OmniSecureStreamType::Connected,
                        16384,
                        None,
                        false,
                        Duration::ZERO,
                        clock(NOW + offset),
                        rng.clone()
                    ),
                    OmniSecureStream::new(accepted, OmniSecureStreamType::Accepted, 16384, None, false, Duration::ZERO, clock(NOW), rng.clone())
                )
            })
            .await?;
            if offset == 0 {
                connected?;
                accepted?;
            } else {
                assert_rejection(accepted, "agreement key outside time tolerance");
            }
        }
        Ok(())
    }
}

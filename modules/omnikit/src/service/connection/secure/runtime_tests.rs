#[cfg(test)]
mod tests {
    use std::{
        convert::Infallible,
        io,
        pin::Pin,
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        },
        task::{Context, Poll, Waker},
        time::Duration,
    };

    use ed25519_dalek::pkcs8::EncodePrivateKey as _;
    use omnius_core_base::error::OmniError as _;
    use parking_lot::Mutex;
    use rand::{SeedableRng, rngs::ChaCha20Rng};
    use rand_core::{CryptoRng, TryCryptoRng, TryRng};
    use serde_json::Value;
    use testresult::TestResult;
    use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};

    use super::super::*;
    use crate::{
        generated::{
            omni_secure::{AuthType, V2AuthMessage, V2FinishedMessage, V2ProfileMessage, V2Role},
            omni_sign::{OmniSignType, OmniSigner},
        },
        prelude::RocketPackStruct,
    };

    fn case(index: usize) -> Value {
        let all: Value = serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/secure_v2_vectors.json"))).unwrap();
        all["cases"][index].clone()
    }

    fn bytes(value: &Value) -> Vec<u8> {
        hex::decode(value.as_str().unwrap()).unwrap()
    }

    // 公開 fixture の再現だけに使う固定 RNG。production には含めない。
    struct FixtureRng {
        bytes: Vec<u8>,
        offset: usize,
    }
    impl TryRng for FixtureRng {
        type Error = Infallible;
        fn try_next_u32(&mut self) -> Result<u32, Infallible> {
            let mut b = [0; 4];
            self.try_fill_bytes(&mut b)?;
            Ok(u32::from_le_bytes(b))
        }
        fn try_next_u64(&mut self) -> Result<u64, Infallible> {
            let mut b = [0; 8];
            self.try_fill_bytes(&mut b)?;
            Ok(u64::from_le_bytes(b))
        }
        fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Infallible> {
            dst.copy_from_slice(&self.bytes[self.offset..self.offset + dst.len()]);
            self.offset += dst.len();
            Ok(())
        }
    }
    impl TryCryptoRng for FixtureRng {}

    fn rng(case: &Value, role: &str) -> Arc<Mutex<dyn CryptoRng + Send + Sync>> {
        let mut data = bytes(&case["inputs"][format!("dh_private_{role}_hex")]);
        data.extend(bytes(&case["inputs"][format!("nonce_{role}_hex")]));
        Arc::new(Mutex::new(FixtureRng { bytes: data, offset: 0 }))
    }

    fn signer(case: &Value, role: &str) -> Arc<OmniSigner> {
        let seed: [u8; 32] = bytes(&case["inputs"][format!("signing_seed_{role}_hex")]).try_into().unwrap();
        Arc::new(OmniSigner {
            typ: OmniSignType::Ed25519_Sha3_256_Base64Url,
            name: case["inputs"][format!("name_{role}")].as_str().unwrap().to_string(),
            key: ed25519_dalek::SigningKey::from_bytes(&seed).to_pkcs8_der().unwrap().as_bytes().to_vec(),
        })
    }

    fn auth(case: &Value, role: &str) -> OmniSecureAuth {
        if case["mode"] == "Mutual" {
            OmniSecureAuth::Mutual { signer: signer(case, role) }
        } else {
            OmniSecureAuth::Anonymous
        }
    }

    fn handshake(case: &Value, role: &str) -> Vec<u8> {
        ["profile", "auth", "finished"]
            .into_iter()
            .flat_map(|kind| bytes(&case["handshake_wire"][format!("{kind}_{role}_hex")]))
            .collect()
    }

    fn frame<M: RocketPackStruct>(message: &M) -> Vec<u8> {
        let body = message.export().unwrap();
        let mut wire = (body.len() as u32).to_le_bytes().to_vec();
        wire.extend(body);
        wire
    }

    #[allow(deprecated)]
    fn authenticated_record(c: &Value, peer: &str, kind: u8, generation: u64, sequence: u64, plaintext: &[u8]) -> Vec<u8> {
        use aes_gcm::{
            Aes256Gcm, KeyInit as _,
            aead::{Aead, Payload},
        };
        use hkdf::SimpleHkdf;
        use sha3::Sha3_256;
        let t2 = bytes(&c["crypto"]["t2_hex"]);
        let direction = [if peer == "i" { 1 } else { 2 }];
        let secret = bytes(&c["crypto"][format!("{peer}_secret_0_hex")]);
        let hkdf = SimpleHkdf::<Sha3_256>::from_prk(&secret).unwrap();
        let suffix = [t2.as_slice(), direction.as_slice(), generation.to_le_bytes().as_slice()].concat();
        let mut key = [0; 32];
        hkdf.expand(&[b"omnius.secure.v2/key\0".as_slice(), &suffix].concat(), &mut key).unwrap();
        let mut iv = [0; 12];
        hkdf.expand(&[b"omnius.secure.v2/iv\0".as_slice(), &suffix].concat(), &mut iv).unwrap();
        for (byte, count) in iv[4..].iter_mut().zip(sequence.to_be_bytes()) {
            *byte ^= count;
        }
        let mut header = vec![kind];
        header.extend(generation.to_le_bytes());
        header.extend(sequence.to_le_bytes());
        header.extend(((plaintext.len() + 16) as u32).to_le_bytes());
        let aad = [b"omnius.secure.v2/record\0".as_slice(), &t2, &direction, &header].concat();
        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
        let ciphertext = cipher.encrypt(aes_gcm::Nonce::from_slice(&iv), Payload { msg: plaintext, aad: &aad }).unwrap();
        header.extend(ciphertext);
        header
    }

    struct Controls {
        written: Mutex<Vec<u8>>,
        max_read: AtomicUsize,
        max_write: AtomicUsize,
        stall_read_at: AtomicUsize,
        stall_write_at: AtomicUsize,
        zero_write: AtomicBool,
        write_error: AtomicBool,
        read_error: AtomicBool,
        stall_flush_at: AtomicUsize,
        flush_error: AtomicBool,
        shutdown_pending: AtomicBool,
        shutdown_error: AtomicBool,
        shutdown_started: AtomicBool,
        strict_shutdown: AtomicBool,
        polls: AtomicUsize,
        poll_budget: AtomicUsize,
        flush_calls: AtomicUsize,
        dropped: AtomicBool,
        waker: Mutex<Option<Waker>>,
    }

    impl Controls {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                written: Mutex::new(vec![]),
                max_read: AtomicUsize::new(usize::MAX),
                max_write: AtomicUsize::new(usize::MAX),
                stall_read_at: AtomicUsize::new(usize::MAX),
                stall_write_at: AtomicUsize::new(usize::MAX),
                zero_write: AtomicBool::new(false),
                write_error: AtomicBool::new(false),
                read_error: AtomicBool::new(false),
                stall_flush_at: AtomicUsize::new(usize::MAX),
                flush_error: AtomicBool::new(false),
                shutdown_pending: AtomicBool::new(false),
                shutdown_error: AtomicBool::new(false),
                shutdown_started: AtomicBool::new(false),
                strict_shutdown: AtomicBool::new(false),
                polls: AtomicUsize::new(0),
                poll_budget: AtomicUsize::new(1_000_000),
                flush_calls: AtomicUsize::new(0),
                dropped: AtomicBool::new(false),
                waker: Mutex::new(None),
            })
        }
        fn release(&self) {
            self.stall_read_at.store(usize::MAX, Ordering::SeqCst);
            self.stall_write_at.store(usize::MAX, Ordering::SeqCst);
            self.stall_flush_at.store(usize::MAX, Ordering::SeqCst);
            self.shutdown_pending.store(false, Ordering::SeqCst);
            if let Some(waker) = self.waker.lock().take() {
                waker.wake();
            }
        }
        fn pending(&self, cx: &Context<'_>) {
            *self.waker.lock() = Some(cx.waker().clone());
        }
        fn check_poll(&self) -> io::Result<()> {
            if self.polls.fetch_add(1, Ordering::SeqCst) >= self.poll_budget.load(Ordering::SeqCst) {
                return Err(io::Error::other("fake I/O poll budget exceeded"));
            }
            Ok(())
        }
        fn arm_budget(&self, calls: usize) {
            self.poll_budget.store(self.polls.load(Ordering::SeqCst) + calls, Ordering::SeqCst);
        }
    }

    struct ScriptIo {
        incoming: Vec<u8>,
        offset: usize,
        control: Arc<Controls>,
    }
    impl Drop for ScriptIo {
        fn drop(&mut self) {
            self.control.dropped.store(true, Ordering::SeqCst);
        }
    }
    impl AsyncRead for ScriptIo {
        fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, output: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
            let this = self.get_mut();
            this.control.check_poll()?;
            if this.control.read_error.load(Ordering::SeqCst) {
                return Poll::Ready(Err(io::Error::new(io::ErrorKind::ConnectionReset, "injected read error")));
            }
            let barrier = this.control.stall_read_at.load(Ordering::SeqCst);
            if this.offset >= barrier {
                this.control.pending(cx);
                return Poll::Pending;
            }
            let n = output
                .remaining()
                .min(this.incoming.len() - this.offset)
                .min(barrier - this.offset)
                .min(this.control.max_read.load(Ordering::SeqCst));
            output.put_slice(&this.incoming[this.offset..this.offset + n]);
            this.offset += n;
            Poll::Ready(Ok(()))
        }
    }
    impl AsyncWrite for ScriptIo {
        fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, input: &[u8]) -> Poll<io::Result<usize>> {
            let c = &self.control;
            c.check_poll()?;
            if c.write_error.load(Ordering::SeqCst) {
                return Poll::Ready(Err(io::Error::new(io::ErrorKind::ConnectionReset, "injected write error")));
            }
            if c.zero_write.load(Ordering::SeqCst) {
                return Poll::Ready(Ok(0));
            }
            let mut written = c.written.lock();
            let barrier = c.stall_write_at.load(Ordering::SeqCst);
            if written.len() >= barrier {
                c.pending(cx);
                return Poll::Pending;
            }
            let n = input.len().min(barrier - written.len()).min(c.max_write.load(Ordering::SeqCst));
            written.extend_from_slice(&input[..n]);
            Poll::Ready(Ok(n))
        }
        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            let c = &self.control;
            c.check_poll()?;
            c.flush_calls.fetch_add(1, Ordering::SeqCst);
            if c.flush_error.load(Ordering::SeqCst) || (c.strict_shutdown.load(Ordering::SeqCst) && c.shutdown_started.load(Ordering::SeqCst)) {
                return Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, "flush after shutdown or injected flush error")));
            }
            if c.written.lock().len() >= c.stall_flush_at.load(Ordering::SeqCst) {
                c.pending(cx);
                return Poll::Pending;
            }
            Poll::Ready(Ok(()))
        }
        fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            let c = &self.control;
            c.check_poll()?;
            c.shutdown_started.store(true, Ordering::SeqCst);
            if c.shutdown_error.load(Ordering::SeqCst) {
                return Poll::Ready(Err(io::Error::new(io::ErrorKind::ConnectionReset, "injected shutdown error")));
            }
            if c.shutdown_pending.load(Ordering::SeqCst) {
                c.pending(cx);
                return Poll::Pending;
            }
            Poll::Ready(Ok(()))
        }
    }

    async fn scripted(case: &Value, incoming: Vec<u8>, control: Arc<Controls>, records: u64) -> crate::result::Result<OmniSecureStream<ScriptIo>> {
        let mut option = OmniSecureStreamOption::new(bytes(&case["inputs"]["context_hex"]))?;
        option.rekey_after_records = records;
        OmniSecureStream::new(
            ScriptIo { incoming, offset: 0, control },
            OmniSecureStreamType::Connected,
            option,
            auth(case, "i"),
            rng(case, "i"),
        )
        .await
    }

    async fn scripted_role(c: &Value, incoming: Vec<u8>, control: Arc<Controls>, role: &str, option: OmniSecureStreamOption) -> crate::result::Result<OmniSecureStream<ScriptIo>> {
        OmniSecureStream::new(
            ScriptIo { incoming, offset: 0, control },
            if role == "i" { OmniSecureStreamType::Connected } else { OmniSecureStreamType::Accepted },
            option,
            auth(c, role),
            rng(c, role),
        )
        .await
    }

    #[tokio::test]
    async fn actual_handshake_and_stream_match_independent_vectors() -> TestResult {
        for index in 0..2 {
            let c = case(index);
            let control = Controls::new();
            control.max_read.store(1, Ordering::SeqCst);
            control.max_write.store(3, Ordering::SeqCst);
            let mut incoming = handshake(&c, "r");
            for name in ["r_data_0", "r_update_0", "r_data_1", "r_close_1"] {
                incoming.extend(bytes(&c["records"][name]["wire_hex"]));
            }
            let mut stream = scripted(&c, incoming, control.clone(), 2).await?;
            assert_eq!(stream.handshake_hash().as_slice(), bytes(&c["crypto"]["t2_hex"]));
            assert_eq!(*control.written.lock(), handshake(&c, "i"));
            if index == 0 {
                assert_eq!(stream.peer_public_key(), Some(signer(&c, "r").public_key()?.as_slice()));
                assert!(stream.peer_cert().is_some());
            } else {
                assert!(stream.peer_cert().is_none());
            }
            let mut empty = [];
            assert_eq!(stream.read(&mut empty).await?, 0);
            assert_eq!(stream.write(&[]).await?, 0);
            stream.flush().await?;
            assert_eq!(*control.written.lock(), handshake(&c, "i"));
            stream.write_all(b"hello i").await?;
            stream.write_all("更新後 i".as_bytes()).await?;
            stream.shutdown().await?;
            stream.shutdown().await?;
            assert_eq!(stream.write(b"late").await.unwrap_err().kind(), io::ErrorKind::BrokenPipe);
            let mut expected = handshake(&c, "i");
            for name in ["i_data_0", "i_update_0", "i_data_1", "i_close_1"] {
                expected.extend(bytes(&c["records"][name]["wire_hex"]));
            }
            assert_eq!(*control.written.lock(), expected);
            let mut read = Vec::new();
            stream.read_to_end(&mut read).await?;
            assert_eq!(read, "hello r更新後 r".as_bytes());
        }
        Ok(())
    }

    #[tokio::test]
    async fn canceled_partial_update_and_shutdown_do_not_duplicate_records() -> TestResult {
        let c = case(0);
        let control = Controls::new();
        let mut stream = scripted(&c, handshake(&c, "r"), control.clone(), 2).await?;
        stream.write_all(b"hello i").await?;
        stream.flush().await?;
        control.stall_write_at.store(control.written.lock().len() + 7, Ordering::SeqCst);
        {
            let mut write = Box::pin(stream.write("更新後 i".as_bytes()));
            assert!(futures_util::poll!(&mut write).is_pending());
        }
        control.release();
        stream.write_all("更新後 i".as_bytes()).await?;
        stream.flush().await?;
        control.stall_write_at.store(control.written.lock().len() + 5, Ordering::SeqCst);
        {
            let mut shutdown = Box::pin(stream.shutdown());
            assert!(futures_util::poll!(&mut shutdown).is_pending());
        }
        assert_eq!(stream.write(b"late").await.unwrap_err().kind(), io::ErrorKind::BrokenPipe);
        control.release();
        stream.shutdown().await?;
        let mut expected = handshake(&c, "i");
        for name in ["i_data_0", "i_update_0", "i_data_1", "i_close_1"] {
            expected.extend(bytes(&c["records"][name]["wire_hex"]));
        }
        assert_eq!(*control.written.lock(), expected);
        Ok(())
    }

    #[tokio::test]
    async fn canceled_partial_record_read_preserves_authenticated_output() -> TestResult {
        let c = case(1);
        for offset in [5, 25] {
            let control = Controls::new();
            let mut incoming = handshake(&c, "r");
            let handshake_length = incoming.len();
            incoming.extend(bytes(&c["records"]["r_data_0"]["wire_hex"]));
            let mut stream = scripted(&c, incoming, control.clone(), 2).await?;
            control.stall_read_at.store(handshake_length + offset, Ordering::SeqCst);
            let mut output = [0xa5; 20];
            {
                let mut read = Box::pin(stream.read(&mut output));
                assert!(futures_util::poll!(&mut read).is_pending());
            }
            assert_eq!(output, [0xa5; 20]);
            control.release();
            let count = stream.read(&mut output).await?;
            assert_eq!(&output[..count], b"hello r");
        }
        Ok(())
    }

    #[tokio::test]
    async fn invalid_record_is_terminal_in_both_directions() -> TestResult {
        for index in 0..2 {
            let c = case(index);
            for invalid in c["negative_records"].as_array().unwrap() {
                // peer fixture の I→R record を Accepted として受ける。
                let control = Controls::new();
                let mut incoming = handshake(&c, "i");
                for name in invalid["before"].as_array().unwrap() {
                    incoming.extend(bytes(&c["records"][name.as_str().unwrap()]["wire_hex"]));
                }
                incoming.extend(bytes(&invalid["wire_hex"]));
                let option = OmniSecureStreamOption::new(bytes(&c["inputs"]["context_hex"]))?;
                let mut stream = OmniSecureStream::new(
                    ScriptIo {
                        incoming,
                        offset: 0,
                        control: control.clone(),
                    },
                    OmniSecureStreamType::Accepted,
                    option,
                    auth(&c, "r"),
                    rng(&c, "r"),
                )
                .await?;
                let mut output = vec![];
                assert!(stream.read_to_end(&mut output).await.is_err(), "{}", invalid["name"]);
                assert!(control.dropped.load(Ordering::SeqCst));
                assert_eq!(stream.write(b"late").await.unwrap_err().kind(), io::ErrorKind::ConnectionAborted);
                assert_eq!(stream.flush().await.unwrap_err().kind(), io::ErrorKind::ConnectionAborted);
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn zero_write_eof_and_transport_error_end_without_spinning() -> TestResult {
        let c = case(1);
        let control = Controls::new();
        let mut stream = scripted(&c, handshake(&c, "r"), control.clone(), 2).await?;
        stream.write_all(b"accepted").await?;
        control.zero_write.store(true, Ordering::SeqCst);
        assert_eq!(stream.flush().await.unwrap_err().kind(), io::ErrorKind::WriteZero);
        assert!(control.dropped.load(Ordering::SeqCst));
        for reset in [false, true] {
            let control = Controls::new();
            let mut stream = scripted(&c, handshake(&c, "r"), control.clone(), 2).await?;
            control.read_error.store(reset, Ordering::SeqCst);
            let mut out = [0; 1];
            let error = stream.read(&mut out).await.unwrap_err();
            assert_eq!(error.kind(), if reset { io::ErrorKind::ConnectionReset } else { io::ErrorKind::UnexpectedEof });
            assert!(control.dropped.load(Ordering::SeqCst));
        }
        Ok(())
    }

    #[tokio::test]
    async fn handshake_rejects_wrong_policy_identity_dh_and_finished() -> TestResult {
        let c = case(0);
        let profile_wire = bytes(&c["handshake_wire"]["profile_r_hex"]);
        let profile = V2ProfileMessage::import(&profile_wire[12..])?;
        for kind in 0..5 {
            let mut peer = profile.clone();
            match kind {
                0 => peer.context = b"another-context".to_vec(),
                1 => peer.role = V2Role::Connected,
                2 => {
                    peer.auth_type = AuthType::None;
                    peer.name.clear();
                    peer.public_key.clear();
                }
                3 => peer.ephemeral_public_key.fill(0),
                4 => {
                    peer.public_key[12..].fill(0);
                }
                _ => unreachable!(),
            }
            let control = Controls::new();
            let mut incoming = b"OMNISC2\0".to_vec();
            incoming.extend(frame(&peer));
            assert!(scripted(&c, incoming, control.clone(), 2).await.is_err());
            assert_eq!(*control.written.lock(), bytes(&c["handshake_wire"]["profile_i_hex"]));
            assert!(control.dropped.load(Ordering::SeqCst));
        }
        for kind in 0..3 {
            let control = Controls::new();
            let mut incoming = profile_wire.clone();
            let original = bytes(&c["handshake_wire"]["auth_r_hex"]);
            let mut peer = V2AuthMessage::import(&original[4..])?;
            match kind {
                0 => peer.cert = None,
                1 => peer.cert.as_mut().unwrap().value[0] ^= 1,
                2 => peer.cert.as_mut().unwrap().name.push('x'),
                _ => unreachable!(),
            }
            incoming.extend(frame(&peer));
            assert!(scripted(&c, incoming, control.clone(), 2).await.is_err());
            let expected = ["profile_i_hex", "auth_i_hex"]
                .into_iter()
                .flat_map(|name| bytes(&c["handshake_wire"][name]))
                .collect::<Vec<_>>();
            assert_eq!(*control.written.lock(), expected);
        }
        let control = Controls::new();
        let mut incoming = profile_wire;
        incoming.extend(bytes(&c["handshake_wire"]["auth_r_hex"]));
        incoming.extend(frame(&V2FinishedMessage { verify_data: vec![0; 32] }));
        assert!(scripted(&c, incoming, control.clone(), 2).await.is_err());
        assert!(control.dropped.load(Ordering::SeqCst));
        Ok(())
    }

    #[tokio::test]
    async fn old_wire_noncanonical_payload_and_oversized_frame_are_rejected() -> TestResult {
        let c = case(1);
        let canonical = bytes(&c["handshake_wire"]["profile_r_hex"]);
        let mut trailing = canonical[12..].to_vec();
        trailing.push(0);
        let mut bad_wire = b"OMNISC2\0".to_vec();
        bad_wire.extend((trailing.len() as u32).to_le_bytes());
        bad_wire.extend(trailing);
        let mut oversized = b"OMNISC2\0".to_vec();
        oversized.extend(16385u32.to_le_bytes());
        for incoming in [b"OLDWIRE!".to_vec(), bad_wire, oversized] {
            let control = Controls::new();
            assert!(scripted(&c, incoming, control.clone(), 2).await.is_err());
            assert!(control.dropped.load(Ordering::SeqCst));
        }
        Ok(())
    }

    #[tokio::test]
    async fn canceled_handshake_discards_transport() -> TestResult {
        let c = case(1);
        let control = Controls::new();
        control.stall_read_at.store(0, Ordering::SeqCst);
        {
            let mut connection = Box::pin(scripted(&c, handshake(&c, "r"), control.clone(), 2));
            assert!(futures_util::poll!(&mut connection).is_pending());
        }
        assert!(control.dropped.load(Ordering::SeqCst));
        Ok(())
    }

    #[tokio::test]
    async fn deep_unknown_enum_payload_is_rejected_before_generated_decode() -> TestResult {
        let c = case(1);
        let canonical = bytes(&c["handshake_wire"]["profile_r_hex"]);
        let mut body = canonical[12..].to_vec();
        let role = body.windows(3).position(|window| window == [0xa1, 2, 0xa0]).unwrap();
        let mut nested = vec![0xa1, 2, 0xa1, 0];
        for _ in 0..1024 {
            nested.extend([0xa1, 0]);
        }
        nested.push(0);
        body.splice(role..role + 3, nested);
        let mut incoming = b"OMNISC2\0".to_vec();
        incoming.extend((body.len() as u32).to_le_bytes());
        incoming.extend(body);
        let control = Controls::new();
        let error = scripted(&c, incoming, control.clone(), 2).await.err().unwrap();
        assert_eq!(error.message(), Some("invalid V2 handshake payload structure"));
        assert!(control.dropped.load(Ordering::SeqCst));
        Ok(())
    }

    #[tokio::test]
    async fn live_mutual_pair_updates_both_directions_on_tiny_transport() -> TestResult {
        let c = case(0);
        let (a, b) = tokio::io::duplex(7);
        let mut option = OmniSecureStreamOption::new(b"live-v2")?;
        option.rekey_after_bytes = 41;
        option.rekey_after_records = 3;
        let random = || -> Arc<Mutex<dyn CryptoRng + Send + Sync>> { Arc::new(Mutex::new(ChaCha20Rng::seed_from_u64(42))) };
        let (a, b) = tokio::time::timeout(Duration::from_secs(10), async {
            tokio::try_join!(
                OmniSecureStream::new(a, OmniSecureStreamType::Connected, option.clone(), auth(&c, "i"), random()),
                OmniSecureStream::new(b, OmniSecureStreamType::Accepted, option, auth(&c, "r"), random())
            )
        })
        .await??;
        assert_eq!(a.handshake_hash(), b.handshake_hash());
        let (mut ar, mut aw) = tokio::io::split(a);
        let (mut br, mut bw) = tokio::io::split(b);
        let a_data = vec![0xab; 1025];
        let b_data = vec![0xcd; 2049];
        let send_a = async {
            aw.write_all(&a_data).await?;
            aw.shutdown().await
        };
        let send_b = async {
            bw.write_all(&b_data).await?;
            bw.shutdown().await
        };
        let read_a = async {
            let mut data = vec![];
            ar.read_to_end(&mut data).await?;
            io::Result::Ok(data)
        };
        let read_b = async {
            let mut data = vec![];
            br.read_to_end(&mut data).await?;
            io::Result::Ok(data)
        };
        let (_, _, received_a, received_b) = tokio::time::timeout(Duration::from_secs(10), async { tokio::try_join!(send_a, send_b, read_a, read_b) }).await??;
        assert_eq!(received_a, b_data);
        assert_eq!(received_b, a_data);
        Ok(())
    }

    #[tokio::test]
    async fn terminal_write_failure_wakes_a_pending_reader() -> TestResult {
        struct CountWake(AtomicUsize);
        impl std::task::Wake for CountWake {
            fn wake(self: Arc<Self>) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        let c = case(1);
        let control = Controls::new();
        let incoming = handshake(&c, "r");
        let end = incoming.len();
        let stream = scripted(&c, incoming, control.clone(), 2).await?;
        control.stall_read_at.store(end, Ordering::SeqCst);
        let (mut reader, mut writer) = tokio::io::split(stream);
        let notified = Arc::new(CountWake(AtomicUsize::new(0)));
        let waker = Waker::from(notified.clone());
        let mut context = Context::from_waker(&waker);
        let mut output = [0; 1];
        let mut read = Box::pin(reader.read(&mut output));
        assert!(std::future::Future::poll(read.as_mut(), &mut context).is_pending());
        writer.write_all(b"queued").await?;
        control.zero_write.store(true, Ordering::SeqCst);
        assert_eq!(writer.flush().await.unwrap_err().kind(), io::ErrorKind::WriteZero);
        assert!(notified.0.load(Ordering::SeqCst) > 0);
        assert_eq!(read.await.unwrap_err().kind(), io::ErrorKind::ConnectionAborted);
        Ok(())
    }

    #[tokio::test]
    async fn constructor_rejects_replayed_transcripts_and_signature_domains() -> TestResult {
        let c = case(0);
        for role in ["i", "r"] {
            let peer_role = if role == "i" { "r" } else { "i" };
            let profile_wire = bytes(&c["handshake_wire"][format!("profile_{peer_role}_hex")]);
            let original_profile = V2ProfileMessage::import(&profile_wire[12..])?;
            let auth_wire = bytes(&c["handshake_wire"][format!("auth_{peer_role}_hex")]);
            let original_auth = V2AuthMessage::import(&auth_wire[4..])?;
            for mutation in 0..9 {
                let mut profile = original_profile.clone();
                let mut peer_auth = original_auth.clone();
                match mutation {
                    0 => profile.nonce[0] ^= 1,
                    1 => profile.ephemeral_public_key = x25519_dalek::PublicKey::from(&x25519_dalek::StaticSecret::from([0x91; 32])).as_bytes().to_vec(),
                    2 => {
                        profile.name.push('x');
                        peer_auth.cert.as_mut().unwrap().name = profile.name.clone();
                    }
                    3 => {
                        let mut alternative = (*signer(&c, peer_role)).clone();
                        alternative.key = ed25519_dalek::SigningKey::from_bytes(&[0xf1; 32]).to_pkcs8_der()?.as_bytes().to_vec();
                        profile.public_key = alternative.public_key()?;
                        // 新しい identity が、別の transcript へ作った正しい署名を流用する。
                        peer_auth.cert = Some(alternative.sign(&bytes(&c["crypto"][format!("{peer_role}_signature_preimage_hex")]))?);
                    }
                    4 => {
                        let t0: [u8; 32] = bytes(&c["crypto"]["t0_hex"]).try_into().unwrap();
                        let reversed = if peer_role == "i" { 2 } else { 1 };
                        peer_auth.cert = Some(signer(&c, peer_role).sign(&super::super::transcript::HandshakeTranscript::signature(reversed, &t0))?);
                    }
                    5 => peer_auth.cert = Some(signer(&c, peer_role).sign(&bytes(&c["crypto"]["t0_hex"]))?),
                    6 => peer_auth.cert.as_mut().unwrap().typ = OmniSignType::None,
                    7 => {
                        peer_auth.cert.as_mut().unwrap().value.pop();
                    }
                    8 => peer_auth.cert.as_mut().unwrap().value.push(0),
                    _ => unreachable!(),
                }
                let mut incoming = b"OMNISC2\0".to_vec();
                incoming.extend(frame(&profile));
                incoming.extend(frame(&peer_auth));
                let control = Controls::new();
                let option = OmniSecureStreamOption::new(bytes(&c["inputs"]["context_hex"]))?;
                assert!(scripted_role(&c, incoming, control.clone(), role, option).await.is_err(), "role={role} mutation={mutation}");
                let expected_length =
                    bytes(&c["handshake_wire"][format!("profile_{role}_hex")]).len() + if role == "i" { bytes(&c["handshake_wire"]["auth_i_hex"]).len() } else { 0 };
                assert_eq!(control.written.lock().len(), expected_length, "no Auth reply or Finished after refusal");
                assert!(control.dropped.load(Ordering::SeqCst));
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn live_dh_replacing_relay_cannot_forward_identity_proofs() -> TestResult {
        use super::super::auth::Authenticator;
        let c = case(0);
        let (i, mut proxy_i) = tokio::io::duplex(4096);
        let (mut proxy_r, r) = tokio::io::duplex(4096);
        let option = OmniSecureStreamOption::new(bytes(&c["inputs"]["context_hex"]))?;
        let initiator = OmniSecureStream::new(i, OmniSecureStreamType::Connected, option.clone(), auth(&c, "i"), rng(&c, "i"));
        let responder = OmniSecureStream::new(r, OmniSecureStreamType::Accepted, option, auth(&c, "r"), rng(&c, "r"));
        let relay = async {
            let mut magic = [0; 8];
            proxy_i.read_exact(&mut magic).await?;
            assert_eq!(&magic, b"OMNISC2\0");
            let mut pi = Authenticator::receive::<V2ProfileMessage>(&mut proxy_i, 16384).await?;
            pi.ephemeral_public_key = x25519_dalek::PublicKey::from(&x25519_dalek::StaticSecret::from([0x91; 32])).as_bytes().to_vec();
            proxy_r.write_all(&magic).await?;
            Authenticator::send(&mut proxy_r, &pi, 16384).await?;
            proxy_r.read_exact(&mut magic).await?;
            let mut pr = Authenticator::receive::<V2ProfileMessage>(&mut proxy_r, 16384).await?;
            pr.ephemeral_public_key = x25519_dalek::PublicKey::from(&x25519_dalek::StaticSecret::from([0x92; 32])).as_bytes().to_vec();
            proxy_i.write_all(&magic).await?;
            Authenticator::send(&mut proxy_i, &pr, 16384).await?;
            let ai = Authenticator::receive::<V2AuthMessage>(&mut proxy_i, 16384).await?;
            Authenticator::send(&mut proxy_r, &ai, 16384).await?;
            let result = Authenticator::receive::<V2AuthMessage>(&mut proxy_r, 16384).await;
            drop(proxy_i);
            drop(proxy_r);
            assert!(result.is_err());
            crate::result::Result::Ok(())
        };
        let (i, r, proxy) = tokio::time::timeout(Duration::from_secs(5), async { tokio::join!(initiator, responder, relay) }).await?;
        assert!(i.is_err());
        assert!(r.is_err());
        proxy?;
        Ok(())
    }

    #[tokio::test]
    async fn profile_validation_is_symmetric_and_precedes_signing() -> TestResult {
        for index in 0..2 {
            let c = case(index);
            for role in ["i", "r"] {
                let peer = if role == "i" { "r" } else { "i" };
                let wire = bytes(&c["handshake_wire"][format!("profile_{peer}_hex")]);
                let original = V2ProfileMessage::import(&wire[12..])?;
                for kind in 0..9 {
                    let mut profile = original.clone();
                    match kind {
                        0 => profile.version = 1,
                        1 => {
                            profile.nonce.pop();
                        }
                        2 => {
                            profile.ephemeral_public_key.pop();
                        }
                        3 => profile.ephemeral_public_key.fill(0),
                        4 => profile.auth_type = if index == 0 { AuthType::None } else { AuthType::Sign },
                        5 if index == 1 => profile.name = "anonymous-name".into(),
                        6 if index == 1 => profile.public_key = signer(&case(0), peer).public_key()?,
                        5 => profile.public_key[0] ^= 1,
                        6 => {
                            // 非正規の y = 2^255-17 の圧縮表現。
                            profile.public_key[12..].fill(0xff);
                            profile.public_key[12] = 0xef;
                            profile.public_key[43] = 0x7f;
                        }
                        7 => profile.context = b"different".to_vec(),
                        8 => profile.role = if role == "i" { V2Role::Connected } else { V2Role::Accepted },
                        _ => unreachable!(),
                    }
                    let mut payload = if kind == 1 || kind == 2 {
                        // schema の長さ制約を持たない独立 CBOR 値で短い field を送る。
                        use crate::prelude::RocketPackEncoder;
                        let mut bytes = Vec::new();
                        let mut encoder = omnius_core_rocketpack::RocketPackBytesEncoder::new(&mut bytes);
                        encoder.write_map(8)?;
                        encoder.write_u64(1)?;
                        encoder.write_u32(profile.version)?;
                        encoder.write_u64(2)?;
                        encoder.write_struct(&profile.role)?;
                        encoder.write_u64(3)?;
                        encoder.write_struct(&profile.auth_type)?;
                        encoder.write_u64(4)?;
                        encoder.write_bytes(&profile.context)?;
                        encoder.write_u64(5)?;
                        encoder.write_bytes(&profile.nonce)?;
                        encoder.write_u64(6)?;
                        encoder.write_bytes(&profile.ephemeral_public_key)?;
                        encoder.write_u64(7)?;
                        encoder.write_string(&profile.name)?;
                        encoder.write_u64(8)?;
                        encoder.write_bytes(&profile.public_key)?;
                        let mut wire = (bytes.len() as u32).to_le_bytes().to_vec();
                        wire.extend(bytes);
                        wire
                    } else {
                        frame(&profile)
                    };
                    let mut incoming = b"OMNISC2\0".to_vec();
                    incoming.append(&mut payload);
                    let control = Controls::new();
                    let option = OmniSecureStreamOption::new(bytes(&c["inputs"]["context_hex"]))?;
                    assert!(scripted_role(&c, incoming, control.clone(), role, option).await.is_err());
                    let permitted = if role == "i" || kind == 3 {
                        bytes(&c["handshake_wire"][format!("profile_{role}_hex")]).len()
                    } else {
                        0
                    };
                    assert_eq!(
                        control.written.lock().len(),
                        permitted,
                        "index={index} role={role} kind={kind}: no signature before validation"
                    );
                    assert!(control.dropped.load(Ordering::SeqCst));
                }
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn anonymous_certificate_is_refused_and_has_no_identity_on_success() -> TestResult {
        let c = case(1);
        for role in ["i", "r"] {
            let peer = if role == "i" { "r" } else { "i" };
            let mut incoming = bytes(&c["handshake_wire"][format!("profile_{peer}_hex")]);
            let signed = case(0);
            incoming.extend(bytes(&signed["handshake_wire"][format!("auth_{peer}_hex")]));
            let control = Controls::new();
            let option = OmniSecureStreamOption::new(bytes(&c["inputs"]["context_hex"]))?;
            assert!(scripted_role(&c, incoming, control.clone(), role, option).await.is_err());
            assert!(control.dropped.load(Ordering::SeqCst));
            let control = Controls::new();
            let option = OmniSecureStreamOption::new(bytes(&c["inputs"]["context_hex"]))?;
            let stream = scripted_role(&c, handshake(&c, peer), control, role, option).await?;
            assert!(stream.peer_cert().is_none());
            assert!(stream.peer_public_key().is_none());
        }
        Ok(())
    }

    #[tokio::test]
    async fn handshake_publication_cancel_and_timeout_cover_every_stage() -> TestResult {
        for index in 0..2 {
            let c = case(index);
            for role in ["i", "r"] {
                let peer = if role == "i" { "r" } else { "i" };
                let input = handshake(&c, peer);
                let output = handshake(&c, role);
                let p = bytes(&c["handshake_wire"][format!("profile_{peer}_hex")]).len();
                let a = p + bytes(&c["handshake_wire"][format!("auth_{peer}_hex")]).len();
                let op = bytes(&c["handshake_wire"][format!("profile_{role}_hex")]).len();
                let oa = op + bytes(&c["handshake_wire"][format!("auth_{role}_hex")]).len();
                let barriers = [
                    (0, 0),
                    (0, 8),
                    (0, 12),
                    (0, p - 1),
                    (0, p),
                    (0, p + 4),
                    (0, a - 1),
                    (0, a),
                    (0, a + 4),
                    (0, input.len() - 1),
                    (1, 3),
                    (1, op - 1),
                    (1, op + 2),
                    (1, oa - 1),
                    (1, oa + 2),
                    (1, output.len() - 1),
                    (2, op),
                    (2, oa),
                    (2, output.len()),
                ];
                for (operation, barrier) in barriers {
                    for timeout in [false, true] {
                        let control = Controls::new();
                        match operation {
                            0 => control.stall_read_at.store(barrier, Ordering::SeqCst),
                            1 => control.stall_write_at.store(barrier, Ordering::SeqCst),
                            2 => control.stall_flush_at.store(barrier, Ordering::SeqCst),
                            _ => unreachable!(),
                        }
                        let option = OmniSecureStreamOption::new(bytes(&c["inputs"]["context_hex"]))?;
                        let mut connection = Box::pin(scripted_role(&c, input.clone(), control.clone(), role, option));
                        assert!(
                            futures_util::poll!(&mut connection).is_pending(),
                            "must not publish: {index} {role} op={operation} barrier={barrier}"
                        );
                        assert!(!control.dropped.load(Ordering::SeqCst));
                        if timeout {
                            assert!(tokio::time::timeout(Duration::from_millis(1), connection).await.is_err());
                        } else {
                            drop(connection);
                        }
                        assert!(control.dropped.load(Ordering::SeqCst));
                    }
                }
                for end in [p, p + 2, a - 1, a, a + 2, input.len() - 1] {
                    let control = Controls::new();
                    control.arm_budget(1000);
                    let option = OmniSecureStreamOption::new(bytes(&c["inputs"]["context_hex"]))?;
                    assert!(scripted_role(&c, input[..end].to_vec(), control.clone(), role, option).await.is_err());
                    assert!(control.dropped.load(Ordering::SeqCst));
                }
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn handshake_rejects_all_noncanonical_profile_and_auth_forms() -> TestResult {
        let c = case(1);
        let canonical = bytes(&c["handshake_wire"]["profile_r_hex"]);
        let body = &canonical[12..];
        let mut unknown = body.to_vec();
        unknown[0] = 0xa9;
        unknown.extend([9, 0]);
        let mut missing = body[..body.len() - 2].to_vec();
        missing[0] = 0xa7;
        let mut nonminimal = body.to_vec();
        nonminimal.splice(2..3, [0x18, 2]);
        let mut reordered = vec![body[0]];
        reordered.extend(&body[3..7]);
        reordered.extend(&body[1..3]);
        reordered.extend(&body[7..]);
        for payload in [unknown, missing, nonminimal, reordered] {
            let mut incoming = b"OMNISC2\0".to_vec();
            incoming.extend((payload.len() as u32).to_le_bytes());
            incoming.extend(payload);
            let control = Controls::new();
            assert!(scripted(&c, incoming, control.clone(), 2).await.is_err());
            assert!(control.dropped.load(Ordering::SeqCst));
        }
        let signed = case(0);
        let original = bytes(&signed["handshake_wire"]["auth_r_hex"]);
        let auth = V2AuthMessage::import(&original[4..])?;
        let mut duplicate = Vec::new();
        {
            use crate::prelude::RocketPackEncoder;
            let mut encoder = omnius_core_rocketpack::RocketPackBytesEncoder::new(&mut duplicate);
            encoder.write_map(2)?;
            for _ in 0..2 {
                encoder.write_u64(1)?;
                encoder.write_struct(auth.cert.as_ref().unwrap())?;
            }
        }
        let mut unknown = original[4..].to_vec();
        unknown[0] = 0xa2;
        unknown.extend([0x18, 99, 0]);
        for payload in [duplicate, unknown] {
            let mut incoming = bytes(&signed["handshake_wire"]["profile_r_hex"]);
            incoming.extend((payload.len() as u32).to_le_bytes());
            incoming.extend(payload);
            let control = Controls::new();
            assert!(scripted(&signed, incoming, control.clone(), 2).await.is_err());
            assert!(control.dropped.load(Ordering::SeqCst));
        }
        let mut incoming = canonical;
        incoming.extend([3, 0, 0, 0, 0xa1, 1, 0xf6]);
        assert!(scripted(&c, incoming, Controls::new(), 2).await.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn configured_handshake_limits_apply_before_body_reads_and_on_send() -> TestResult {
        for index in 0..2 {
            let c = case(index);
            for role in ["i", "r"] {
                let peer = if role == "i" { "r" } else { "i" };
                let p = bytes(&c["handshake_wire"][format!("profile_{peer}_hex")]);
                let a = bytes(&c["handshake_wire"][format!("auth_{peer}_hex")]);
                for stage in 0..3 {
                    let mut incoming = match stage {
                        0 => b"OMNISC2\0".to_vec(),
                        1 => p.clone(),
                        2 => {
                            let mut v = p.clone();
                            v.extend(&a);
                            v
                        }
                        _ => unreachable!(),
                    };
                    incoming.extend(513u32.to_le_bytes());
                    let control = Controls::new();
                    control.stall_read_at.store(incoming.len(), Ordering::SeqCst);
                    let mut option = OmniSecureStreamOption::new(bytes(&c["inputs"]["context_hex"]))?;
                    option.handshake_max_frame_length = 512;
                    let mut connection = Box::pin(scripted_role(&c, incoming, control.clone(), role, option));
                    assert!(
                        matches!(futures_util::poll!(&mut connection), Poll::Ready(Err(_))),
                        "must refuse header without reading body"
                    );
                    drop(connection);
                    assert!(control.dropped.load(Ordering::SeqCst));
                }
                let pi = bytes(&c["handshake_wire"]["profile_i_hex"]);
                let pr = bytes(&c["handshake_wire"]["profile_r_hex"]);
                let mut option = OmniSecureStreamOption::new(bytes(&c["inputs"]["context_hex"]))?;
                option.handshake_max_frame_length = pi.len().max(pr.len()) - 12;
                let control = Controls::new();
                let stream = scripted_role(&c, handshake(&c, peer), control.clone(), role, option).await?;
                assert_eq!(*control.written.lock(), handshake(&c, role));
                drop(stream);
            }
            let control = Controls::new();
            let mut option = OmniSecureStreamOption::new(bytes(&c["inputs"]["context_hex"]))?;
            option.handshake_max_frame_length = bytes(&c["handshake_wire"]["profile_i_hex"]).len() - 13;
            assert!(scripted_role(&c, handshake(&c, "r"), control.clone(), "i", option).await.is_err());
            assert_eq!(*control.written.lock(), b"OMNISC2\0");
        }
        let c = case(1);
        for kind in 0..8 {
            let mut option = OmniSecureStreamOption::new(bytes(&c["inputs"]["context_hex"]))?;
            match kind {
                0 => option.context.clear(),
                1 => option.context = vec![0; 257],
                2 => option.handshake_max_frame_length = 0,
                3 => option.handshake_max_frame_length = 16385,
                4 => option.rekey_after_bytes = 8,
                5 => option.rekey_after_bytes = (1 << 30) + 1,
                6 => option.rekey_after_records = 1,
                7 => option.rekey_after_records = (1 << 20) + 1,
                _ => unreachable!(),
            }
            let control = Controls::new();
            assert!(scripted_role(&c, handshake(&c, "r"), control.clone(), "i", option).await.is_err());
            assert_eq!(control.polls.load(Ordering::SeqCst), 0);
            assert!(control.dropped.load(Ordering::SeqCst));
        }
        Ok(())
    }

    #[tokio::test]
    async fn key_update_tampering_replay_and_old_epoch_data_are_terminal() -> TestResult {
        for index in 0..2 {
            let c = case(index);
            let data = bytes(&c["records"]["r_data_0"]["wire_hex"]);
            let update = bytes(&c["records"]["r_update_0"]["wire_hex"]);
            for kind in 0..7 {
                let mut incoming = handshake(&c, "r");
                incoming.extend(&data);
                let mut attack = update.clone();
                match kind {
                    0 => *attack.last_mut().unwrap() ^= 1,
                    1 => attack[1..9].copy_from_slice(&1u64.to_le_bytes()),
                    2 => attack[9..17].copy_from_slice(&2u64.to_le_bytes()),
                    3 => incoming.extend(&update),
                    4 => {
                        incoming.extend(&update);
                        attack = data.clone();
                    }
                    5 => attack = authenticated_record(&c, "r", 2, 0, 1, &0u64.to_le_bytes()),
                    6 => attack = authenticated_record(&c, "r", 2, 0, 1, &2u64.to_le_bytes()),
                    _ => unreachable!(),
                }
                incoming.extend(attack);
                let control = Controls::new();
                let mut stream = scripted(&c, incoming, control.clone(), 2).await?;
                let mut output = Vec::new();
                assert_eq!(stream.read_to_end(&mut output).await.unwrap_err().kind(), io::ErrorKind::InvalidData);
                assert_eq!(output, b"hello r");
                assert!(control.dropped.load(Ordering::SeqCst));
                assert_eq!(stream.write(b"forbidden").await.unwrap_err().kind(), io::ErrorKind::ConnectionAborted);
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn receiver_usage_reservations_and_epoch_exhaustion_end_the_stream() -> TestResult {
        let c = case(1);
        for (generation, sequence, used, kind, payload) in [
            (0, 1, (1u64 << 30) - 8, 1, vec![1]),
            (0, (1u64 << 20) - 1, 0, 1, vec![1]),
            (0, 1, (1u64 << 30) - 7, 2, 1u64.to_le_bytes().to_vec()),
            (0, 1u64 << 20, 0, 3, vec![]),
            (u64::MAX, 1, 0, 2, 0u64.to_le_bytes().to_vec()),
        ] {
            let mut incoming = handshake(&c, "r");
            incoming.extend(authenticated_record(&c, "r", kind, generation, sequence, &payload));
            let control = Controls::new();
            let mut stream = scripted(&c, incoming, control.clone(), 2).await?;
            stream.set_test_recv_state(generation, sequence, used);
            let mut output = [0xa5; 1];
            assert_eq!(stream.read(&mut output).await.unwrap_err().kind(), io::ErrorKind::InvalidData);
            assert_eq!(output, [0xa5]);
            assert!(control.dropped.load(Ordering::SeqCst));
            assert_eq!(stream.flush().await.unwrap_err().kind(), io::ErrorKind::ConnectionAborted);
        }
        let control = Controls::new();
        let mut stream = scripted(&c, handshake(&c, "r"), control.clone(), 2).await?;
        stream.set_test_send_state(u64::MAX, 1, 1);
        let before = control.written.lock().clone();
        assert_eq!(stream.write(b"must not wrap").await.unwrap_err().kind(), io::ErrorKind::InvalidData);
        assert_eq!(*control.written.lock(), before);
        assert!(control.dropped.load(Ordering::SeqCst));
        Ok(())
    }

    #[tokio::test]
    async fn terminal_read_failure_wakes_a_pending_writer() -> TestResult {
        struct CountWake(AtomicUsize);
        impl std::task::Wake for CountWake {
            fn wake(self: Arc<Self>) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        let c = case(1);
        let mut incoming = handshake(&c, "r");
        let mut bad = bytes(&c["records"]["r_data_0"]["wire_hex"]);
        *bad.last_mut().unwrap() ^= 1;
        incoming.extend(bad);
        let control = Controls::new();
        let stream = scripted(&c, incoming, control.clone(), 2).await?;
        let (mut reader, mut writer) = tokio::io::split(stream);
        writer.write_all(b"queued").await?;
        control.stall_write_at.store(control.written.lock().len() + 5, Ordering::SeqCst);
        let notified = Arc::new(CountWake(AtomicUsize::new(0)));
        let waker = Waker::from(notified.clone());
        let mut context = Context::from_waker(&waker);
        let mut flush = Box::pin(writer.flush());
        assert!(std::future::Future::poll(flush.as_mut(), &mut context).is_pending());
        let mut output = [0xa5; 10];
        assert_eq!(reader.read(&mut output).await.unwrap_err().kind(), io::ErrorKind::InvalidData);
        assert_eq!(output, [0xa5; 10]);
        assert!(notified.0.load(Ordering::SeqCst) > 0);
        assert_eq!(flush.await.unwrap_err().kind(), io::ErrorKind::ConnectionAborted);
        Ok(())
    }

    #[tokio::test]
    async fn shutdown_cancel_preserves_accepted_data_at_all_output_phases() -> TestResult {
        let c = case(0);
        for phase in 0..5 {
            let control = Controls::new();
            let mut stream = scripted(&c, handshake(&c, "r"), control.clone(), 2).await?;
            stream.write_all(b"hello i").await?;
            // DATA は Ready で受け付け済み。flush 未完了の状態から shutdown を始める。
            let base = control.written.lock().len();
            let data_len = bytes(&c["records"]["i_data_0"]["wire_hex"]).len();
            match phase {
                0 => control.stall_write_at.store(base + 5, Ordering::SeqCst),
                1 => control.stall_write_at.store(base + data_len + 5, Ordering::SeqCst),
                2 => control.stall_flush_at.store(base + data_len + 37, Ordering::SeqCst),
                3 | 4 => {
                    control.strict_shutdown.store(true, Ordering::SeqCst);
                    control.shutdown_pending.store(true, Ordering::SeqCst);
                }
                _ => unreachable!(),
            }
            {
                let mut end = Box::pin(stream.shutdown());
                assert!(futures_util::poll!(&mut end).is_pending());
            }
            assert_eq!(stream.write(b"new").await.unwrap_err().kind(), io::ErrorKind::BrokenPipe);
            control.release();
            if phase == 4 {
                stream.flush().await?;
            }
            stream.shutdown().await?;
            stream.shutdown().await?;
            let mut expected = handshake(&c, "i");
            expected.extend(bytes(&c["records"]["i_data_0"]["wire_hex"]));
            expected.extend(authenticated_record(&c, "i", 3, 0, 1, &[]));
            assert_eq!(*control.written.lock(), expected);
        }
        Ok(())
    }

    #[tokio::test]
    async fn output_faults_are_terminal_and_bounded_for_every_record_kind() -> TestResult {
        let c = case(1);
        for phase in 0..3 {
            for zero in [false, true] {
                let control = Controls::new();
                let mut stream = scripted(&c, handshake(&c, "r"), control.clone(), 2).await?;
                stream.write_all(b"hello i").await?;
                if phase != 0 {
                    stream.flush().await?;
                }
                control.arm_budget(1);
                if zero {
                    control.zero_write.store(true, Ordering::SeqCst);
                } else {
                    control.write_error.store(true, Ordering::SeqCst);
                }
                let error = match phase {
                    0 => stream.flush().await.unwrap_err(),
                    1 => stream.write(b"next").await.unwrap_err(),
                    2 => stream.shutdown().await.unwrap_err(),
                    _ => unreachable!(),
                };
                assert_eq!(error.kind(), if zero { io::ErrorKind::WriteZero } else { io::ErrorKind::ConnectionReset });
                assert!(control.dropped.load(Ordering::SeqCst));
            }
        }
        for shutdown in [false, true] {
            let control = Controls::new();
            let mut stream = scripted(&c, handshake(&c, "r"), control.clone(), 2).await?;
            stream.write_all(b"accepted").await?;
            control.arm_budget(4);
            if shutdown {
                control.shutdown_error.store(true, Ordering::SeqCst);
            } else {
                control.flush_error.store(true, Ordering::SeqCst);
            }
            let error = stream.shutdown().await.unwrap_err();
            assert_eq!(error.kind(), if shutdown { io::ErrorKind::ConnectionReset } else { io::ErrorKind::BrokenPipe });
            assert!(control.dropped.load(Ordering::SeqCst));
        }
        Ok(())
    }

    #[tokio::test]
    async fn truncation_of_data_update_and_close_is_not_normal_eof() -> TestResult {
        let c = case(0);
        for (name, before) in [
            ("r_data_0", vec![]),
            ("r_update_0", vec!["r_data_0"]),
            ("r_close_1", vec!["r_data_0", "r_update_0", "r_data_1"]),
        ] {
            let wire = bytes(&c["records"][name]["wire_hex"]);
            for cut in [0, 1, 20, 21, wire.len() - 8, wire.len() - 1] {
                let mut incoming = handshake(&c, "r");
                for prefix in &before {
                    incoming.extend(bytes(&c["records"][*prefix]["wire_hex"]));
                }
                incoming.extend(&wire[..cut]);
                let control = Controls::new();
                let mut stream = scripted(&c, incoming, control.clone(), 2).await?;
                control.arm_budget(20);
                let mut output = vec![];
                assert_eq!(stream.read_to_end(&mut output).await.unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
                assert!(control.dropped.load(Ordering::SeqCst));
            }
        }
        let mut incoming = handshake(&c, "r");
        for name in ["r_data_0", "r_update_0", "r_data_1"] {
            incoming.extend(bytes(&c["records"][name]["wire_hex"]));
        }
        let mut close = bytes(&c["records"]["r_close_1"]["wire_hex"]);
        *close.last_mut().unwrap() ^= 1;
        incoming.extend(close);
        let mut stream = scripted(&c, incoming, Controls::new(), 2).await?;
        let mut output = vec![];
        assert_eq!(stream.read_to_end(&mut output).await.unwrap_err().kind(), io::ErrorKind::InvalidData);
        assert_eq!(output, "hello r更新後 r".as_bytes());
        Ok(())
    }

    #[tokio::test]
    async fn maximum_data_records_split_and_unread_suffix_survives_until_close() -> TestResult {
        let c = case(1);
        let payload = vec![0x77; 65537];
        let mut incoming = handshake(&c, "r");
        incoming.extend(authenticated_record(&c, "r", 1, 0, 0, &payload[..65536]));
        incoming.extend(authenticated_record(&c, "r", 1, 0, 1, &payload[65536..]));
        incoming.extend(authenticated_record(&c, "r", 3, 0, 2, &[]));
        let control = Controls::new();
        let mut stream = scripted(&c, incoming, control.clone(), 1 << 20).await?;
        stream.write_all(&payload).await?;
        stream.shutdown().await?;
        let mut expected = handshake(&c, "i");
        expected.extend(authenticated_record(&c, "i", 1, 0, 0, &payload[..65536]));
        expected.extend(authenticated_record(&c, "i", 1, 0, 1, &payload[65536..]));
        expected.extend(authenticated_record(&c, "i", 3, 0, 2, &[]));
        assert_eq!(*control.written.lock(), expected);
        let mut received = Vec::new();
        let mut buffer = [0; 3];
        loop {
            let count = stream.read(&mut buffer).await?;
            if count == 0 {
                break;
            }
            received.extend_from_slice(&buffer[..count]);
        }
        assert_eq!(received, payload);
        let mut bad = bytes(&c["records"]["r_data_0"]["wire_hex"])[..21].to_vec();
        bad[17..21].copy_from_slice(&65553u32.to_le_bytes());
        let mut incoming = handshake(&c, "r");
        incoming.extend(bad);
        let control = Controls::new();
        control.stall_read_at.store(incoming.len(), Ordering::SeqCst);
        let mut stream = scripted(&c, incoming, control.clone(), 2).await?;
        let mut output = [0xa5; 1];
        let mut read = Box::pin(stream.read(&mut output));
        assert!(matches!(futures_util::poll!(&mut read), Poll::Ready(Err(_))));
        drop(read);
        assert_eq!(output, [0xa5]);
        assert!(control.dropped.load(Ordering::SeqCst));
        Ok(())
    }

    #[tokio::test]
    async fn application_frames_cross_updates_inside_headers_and_payloads() -> TestResult {
        use crate::service::connection::codec::{FramedReceiver, FramedRecv, FramedSend, FramedSender};
        use tokio_util::bytes::Bytes;
        let c = case(0);
        let (a, b) = tokio::io::duplex(7);
        let mut option = OmniSecureStreamOption::new(b"framed-v2")?;
        option.rekey_after_bytes = 9;
        option.rekey_after_records = 2;
        let (a, b) = tokio::time::timeout(Duration::from_secs(10), async {
            tokio::try_join!(
                OmniSecureStream::new(a, OmniSecureStreamType::Connected, option.clone(), auth(&c, "i"), rng(&c, "i")),
                OmniSecureStream::new(b, OmniSecureStreamType::Accepted, option, auth(&c, "r"), rng(&c, "r"))
            )
        })
        .await??;
        let (ar, aw) = tokio::io::split(a);
        let (br, bw) = tokio::io::split(b);
        let mut ar = FramedReceiver::new(ar, 4096);
        let mut br = FramedReceiver::new(br, 4096);
        let mut aw = FramedSender::new(aw, 4096);
        let mut bw = FramedSender::new(bw, 4096);
        let a_frames = [Bytes::from(vec![0xa1; 65]), Bytes::from(vec![0xa2; 257])];
        let b_frames = [Bytes::from(vec![0xb1; 129]), Bytes::from(vec![0xb2; 513])];
        let send_a = async {
            for frame in &a_frames {
                aw.send(frame.clone()).await?;
            }
            aw.into_inner().shutdown().await?;
            crate::result::Result::Ok(())
        };
        let send_b = async {
            for frame in &b_frames {
                bw.send(frame.clone()).await?;
            }
            bw.into_inner().shutdown().await?;
            crate::result::Result::Ok(())
        };
        let read_a = async {
            for frame in &b_frames {
                assert_eq!(ar.recv().await?, *frame);
            }
            assert!(ar.recv().await.is_err());
            crate::result::Result::Ok(())
        };
        let read_b = async {
            for frame in &a_frames {
                assert_eq!(br.recv().await?, *frame);
            }
            assert!(br.recv().await.is_err());
            crate::result::Result::Ok(())
        };
        tokio::time::timeout(Duration::from_secs(10), async { tokio::try_join!(send_a, send_b, read_a, read_b) }).await??;
        Ok(())
    }
}

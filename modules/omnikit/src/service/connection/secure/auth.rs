use std::sync::Arc;

use ed25519_dalek::{VerifyingKey, pkcs8::DecodePublicKey as _};
use parking_lot::Mutex;
use rand_core::CryptoRng;
use subtle::ConstantTimeEq;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use zeroize::Zeroizing;

use crate::{
    generated::{omni_secure::*, omni_sign::OmniCert},
    prelude::*,
};

use super::{handshake_payload::HandshakePayload, kdf::V2Kdf, settings::*, transcript::HandshakeTranscript};

pub(super) struct Authenticator;

pub(super) struct AuthResult {
    pub peer_cert: Option<OmniCert>,
    pub transcript: [u8; 32],
    pub send_secret: Zeroizing<[u8; 32]>,
    pub recv_secret: Zeroizing<[u8; 32]>,
}

impl Authenticator {
    pub async fn authenticate<R, W>(
        reader: &mut R,
        writer: &mut W,
        typ: OmniSecureStreamType,
        auth: &OmniSecureAuth,
        option: &OmniSecureStreamOption,
        rng: Arc<Mutex<dyn CryptoRng + Send + Sync>>,
    ) -> Result<AuthResult>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        let mut seed = Zeroizing::new([0u8; 32]);
        let mut nonce = [0u8; 32];
        {
            let mut rng = rng.lock();
            rng.fill_bytes(seed.as_mut());
            rng.fill_bytes(&mut nonce);
        }
        let private_key = x25519_dalek::StaticSecret::from(*seed);
        drop(seed);
        let ephemeral_public_key = x25519_dalek::PublicKey::from(&private_key).as_bytes().to_vec();
        let (name, public_key) = match auth {
            OmniSecureAuth::Anonymous => (String::new(), Vec::new()),
            OmniSecureAuth::Mutual { signer } => (signer.name.clone(), signer.public_key()?),
        };
        let my_profile = V2ProfileMessage {
            version: V2_VERSION,
            role: typ.role(),
            auth_type: auth.auth_type(),
            context: option.context.clone(),
            nonce: nonce.to_vec(),
            ephemeral_public_key,
            name,
            public_key,
        };
        Self::validate_profile(&my_profile, &typ.role(), auth, option)?;
        let peer_profile = match typ {
            OmniSecureStreamType::Connected => {
                writer.write_all(b"OMNISC2\0").await?;
                Self::send(writer, &my_profile, option.handshake_max_frame_length).await?;
                Self::receive_magic(reader).await?;
                let peer = Self::receive::<V2ProfileMessage>(reader, option.handshake_max_frame_length).await?;
                Self::validate_profile(&peer, &V2Role::Accepted, auth, option)?;
                peer
            }
            OmniSecureStreamType::Accepted => {
                Self::receive_magic(reader).await?;
                let peer = Self::receive::<V2ProfileMessage>(reader, option.handshake_max_frame_length).await?;
                Self::validate_profile(&peer, &V2Role::Connected, auth, option)?;
                writer.write_all(b"OMNISC2\0").await?;
                Self::send(writer, &my_profile, option.handshake_max_frame_length).await?;
                peer
            }
        };
        let public: [u8; 32] = peer_profile.ephemeral_public_key.as_slice().try_into().map_err(|_| Self::invalid("DH public key length"))?;
        let shared = private_key.diffie_hellman(&x25519_dalek::PublicKey::from(public));
        drop(private_key);
        if !shared.was_contributory() {
            return Err(Self::invalid("non-contributory shared secret"));
        }
        let (i_profile, r_profile) = match typ {
            OmniSecureStreamType::Connected => (&my_profile, &peer_profile),
            OmniSecureStreamType::Accepted => (&peer_profile, &my_profile),
        };
        let t0 = HandshakeTranscript::hello(i_profile, r_profile);
        let prk = V2Kdf::hmac(&t0, &[shared.as_bytes()]);
        drop(shared);
        let (my_auth, peer_auth) = match typ {
            OmniSecureStreamType::Connected => {
                let mine = Self::sign(auth, &my_profile, &t0)?;
                Self::send(writer, &mine, option.handshake_max_frame_length).await?;
                let peer = Self::receive::<V2AuthMessage>(reader, option.handshake_max_frame_length).await?;
                HandshakeTranscript::validate_auth(&peer_profile, &peer, &t0)?;
                (mine, peer)
            }
            OmniSecureStreamType::Accepted => {
                let peer = Self::receive::<V2AuthMessage>(reader, option.handshake_max_frame_length).await?;
                HandshakeTranscript::validate_auth(&peer_profile, &peer, &t0)?;
                let mine = Self::sign(auth, &my_profile, &t0)?;
                Self::send(writer, &mine, option.handshake_max_frame_length).await?;
                (mine, peer)
            }
        };
        let (i_auth, r_auth) = match typ {
            OmniSecureStreamType::Connected => (&my_auth, &peer_auth),
            OmniSecureStreamType::Accepted => (&peer_auth, &my_auth),
        };
        let t1 = HandshakeTranscript::authenticated(&t0, i_auth, r_auth);
        let fi = V2Kdf::expand::<32>(prk.as_slice(), &[b"omnius.secure.v2/finished/initiator\0", &t1]);
        let fr = V2Kdf::expand::<32>(prk.as_slice(), &[b"omnius.secure.v2/finished/responder\0", &t1]);
        let vi = V2Kdf::hmac(fi.as_slice(), &[b"omnius.secure.v2/verify/initiator\0", &t1]);
        let vr = V2Kdf::hmac(fr.as_slice(), &[b"omnius.secure.v2/verify/responder\0", &t1, vi.as_slice()]);
        match typ {
            OmniSecureStreamType::Connected => {
                Self::send(writer, &V2FinishedMessage { verify_data: vi.to_vec() }, option.handshake_max_frame_length).await?;
                Self::verify_finished(reader, vr.as_slice(), option.handshake_max_frame_length).await?;
            }
            OmniSecureStreamType::Accepted => {
                Self::verify_finished(reader, vi.as_slice(), option.handshake_max_frame_length).await?;
                Self::send(writer, &V2FinishedMessage { verify_data: vr.to_vec() }, option.handshake_max_frame_length).await?;
            }
        }
        let transcript = HandshakeTranscript::session(&t1, vi.as_slice(), vr.as_slice());
        let si = V2Kdf::expand::<32>(prk.as_slice(), &[b"omnius.secure.v2/traffic/initiator-to-responder\0", &transcript]);
        let sr = V2Kdf::expand::<32>(prk.as_slice(), &[b"omnius.secure.v2/traffic/responder-to-initiator\0", &transcript]);
        let (send_secret, recv_secret) = match typ {
            OmniSecureStreamType::Connected => (si, sr),
            OmniSecureStreamType::Accepted => (sr, si),
        };
        Ok(AuthResult {
            peer_cert: peer_auth.cert,
            transcript,
            send_secret,
            recv_secret,
        })
    }

    fn sign(auth: &OmniSecureAuth, profile: &V2ProfileMessage, t0: &[u8; 32]) -> Result<V2AuthMessage> {
        let cert = match auth {
            OmniSecureAuth::Anonymous => None,
            OmniSecureAuth::Mutual { signer } => Some(signer.sign(&HandshakeTranscript::signature(HandshakeTranscript::role_tag(&profile.role), t0))?),
        };
        let message = V2AuthMessage { cert };
        HandshakeTranscript::validate_auth(profile, &message, t0)?;
        Ok(message)
    }

    fn validate_profile(profile: &V2ProfileMessage, role: &V2Role, auth: &OmniSecureAuth, option: &OmniSecureStreamOption) -> Result<()> {
        V2ProfileMessage::validate(profile)?;
        if profile.version != V2_VERSION || &profile.role != role || profile.auth_type != auth.auth_type() || profile.context != option.context {
            return Err(Self::invalid("V2 profile does not match configured policy"));
        }
        match auth {
            OmniSecureAuth::Anonymous if profile.name.is_empty() && profile.public_key.is_empty() => Ok(()),
            OmniSecureAuth::Mutual { .. } => {
                if profile.public_key.len() != 44 || profile.public_key[..12] != [0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00] {
                    return Err(Self::invalid("noncanonical Ed25519 DER public key"));
                }
                let key = VerifyingKey::from_public_key_der(&profile.public_key)?;
                if key.is_weak() || key.to_edwards().compress().as_bytes() != key.as_bytes() {
                    return Err(Self::invalid("weak or noncanonical Ed25519 public key"));
                }
                Ok(())
            }
            _ => Err(Self::invalid("anonymous identity must be empty")),
        }
    }

    async fn receive_magic<R: AsyncRead + Unpin>(reader: &mut R) -> Result<()> {
        let mut magic = [0u8; 8];
        reader.read_exact(&mut magic).await?;
        if &magic != b"OMNISC2\0" {
            return Err(Self::invalid("unsupported secure stream wire format"));
        }
        Ok(())
    }

    pub(super) async fn send<W: AsyncWrite + Unpin, M: RocketPackStruct>(writer: &mut W, message: &M, limit: usize) -> Result<()> {
        let bytes = message.export()?;
        if bytes.len() > limit {
            return Err(Self::invalid("handshake frame exceeds limit"));
        }
        writer.write_all(&(bytes.len() as u32).to_le_bytes()).await?;
        writer.write_all(&bytes).await?;
        writer.flush().await?;
        Ok(())
    }

    pub(super) async fn receive<M: RocketPackStruct>(reader: &mut (impl AsyncRead + Unpin), limit: usize) -> Result<M> {
        let length = reader.read_u32_le().await? as usize;
        if length > limit {
            return Err(Self::invalid("handshake frame exceeds limit"));
        }
        let mut bytes = vec![0; length];
        reader.read_exact(&mut bytes).await?;
        HandshakePayload::validate(&bytes)?;
        let message = M::import(&bytes)?;
        if message.export()? != bytes {
            return Err(Self::invalid("noncanonical handshake payload"));
        }
        Ok(message)
    }

    async fn verify_finished(reader: &mut (impl AsyncRead + Unpin), expected: &[u8], limit: usize) -> Result<()> {
        let message = Self::receive::<V2FinishedMessage>(reader, limit).await?;
        if !bool::from(message.verify_data.as_slice().ct_eq(expected)) {
            return Err(Self::invalid("invalid Finished"));
        }
        Ok(())
    }

    fn invalid(message: &'static str) -> Error {
        Error::new(ErrorKind::InvalidFormat).with_message(message)
    }
}

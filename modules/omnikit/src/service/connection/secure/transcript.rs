use crate::{
    generated::{
        omni_secure::{V2AuthMessage, V2ProfileMessage, V2Role},
        omni_sign::OmniSignType,
    },
    prelude::*,
};

use super::kdf::V2Kdf;

pub(super) struct HandshakeTranscript;

impl HandshakeTranscript {
    pub fn role_tag(role: &V2Role) -> u8 {
        match role {
            V2Role::Connected => 1,
            V2Role::Accepted => 2,
        }
    }
    fn append_length_prefixed(buffer: &mut Vec<u8>, bytes: &[u8]) {
        buffer.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        buffer.extend_from_slice(bytes);
    }

    pub fn profile(profile: &V2ProfileMessage) -> Vec<u8> {
        let mut result = profile.version.to_le_bytes().to_vec();
        result.push(Self::role_tag(&profile.role));
        result.push(profile.auth_type.to_u32() as u8);
        Self::append_length_prefixed(&mut result, &profile.context);
        result.extend_from_slice(&profile.nonce);
        result.extend_from_slice(&profile.ephemeral_public_key);
        Self::append_length_prefixed(&mut result, profile.name.as_bytes());
        Self::append_length_prefixed(&mut result, &profile.public_key);
        result
    }

    pub fn auth(auth: &V2AuthMessage) -> Vec<u8> {
        let Some(cert) = &auth.cert else {
            return vec![0];
        };
        let mut result = vec![1];
        let typ: u32 = match cert.typ {
            OmniSignType::None => 1,
            OmniSignType::Ed25519_Sha3_256_Base64Url => 2,
        };
        result.extend_from_slice(&typ.to_le_bytes());
        Self::append_length_prefixed(&mut result, cert.name.as_bytes());
        Self::append_length_prefixed(&mut result, &cert.public_key);
        Self::append_length_prefixed(&mut result, &cert.value);
        result
    }

    pub fn hello(i: &V2ProfileMessage, r: &V2ProfileMessage) -> [u8; 32] {
        let mut bytes = b"omnius.secure.v2/hello\0".to_vec();
        Self::append_length_prefixed(&mut bytes, &Self::profile(i));
        Self::append_length_prefixed(&mut bytes, &Self::profile(r));
        V2Kdf::hash(&[&bytes])
    }

    pub fn signature(role: u8, t0: &[u8; 32]) -> Vec<u8> {
        let mut bytes = b"omnius.secure.v2/signature\0".to_vec();
        bytes.push(role);
        bytes.extend_from_slice(t0);
        bytes
    }

    pub fn authenticated(t0: &[u8; 32], i: &V2AuthMessage, r: &V2AuthMessage) -> [u8; 32] {
        let mut bytes = b"omnius.secure.v2/auth\0".to_vec();
        bytes.extend_from_slice(t0);
        Self::append_length_prefixed(&mut bytes, &Self::auth(i));
        Self::append_length_prefixed(&mut bytes, &Self::auth(r));
        V2Kdf::hash(&[&bytes])
    }

    pub fn session(t1: &[u8; 32], vi: &[u8], vr: &[u8]) -> [u8; 32] {
        V2Kdf::hash(&[b"omnius.secure.v2/session\0", t1, vi, vr])
    }

    pub fn validate_auth(profile: &V2ProfileMessage, auth: &V2AuthMessage, t0: &[u8; 32]) -> Result<()> {
        match (&profile.auth_type, &auth.cert) {
            (crate::generated::omni_secure::AuthType::None, None) => Ok(()),
            (crate::generated::omni_secure::AuthType::Sign, Some(cert))
                if cert.typ == OmniSignType::Ed25519_Sha3_256_Base64Url && cert.name == profile.name && cert.public_key == profile.public_key && cert.value.len() == 64 =>
            {
                cert.verify(&Self::signature(Self::role_tag(&profile.role), t0))
            }
            _ => Err(Error::new(ErrorKind::InvalidFormat).with_message("V2 authentication does not match profile")),
        }
    }
}

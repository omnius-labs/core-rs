use std::sync::Arc;

use crate::{
    generated::{omni_secure::*, omni_sign::OmniSigner},
    prelude::*,
};

/// 認証の強度を peer の申告から選び直さないための明示設定。
#[derive(Clone)]
pub enum OmniSecureAuth {
    Anonymous,
    Mutual { signer: Arc<OmniSigner> },
}

impl std::fmt::Debug for OmniSecureAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Anonymous => "Anonymous",
            Self::Mutual { .. } => "Mutual { signer: <redacted> }",
        })
    }
}

impl OmniSecureAuth {
    pub(super) fn auth_type(&self) -> AuthType {
        match self {
            Self::Anonymous => AuthType::None,
            Self::Mutual { .. } => AuthType::Sign,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OmniSecureStreamType {
    Connected,
    Accepted,
}

impl OmniSecureStreamType {
    pub(super) fn role(self) -> V2Role {
        match self {
            Self::Connected => V2Role::Connected,
            Self::Accepted => V2Role::Accepted,
        }
    }
}

/// context は必須。更新しきい値は送信側にだけ適用する。
#[derive(Debug, Clone)]
pub struct OmniSecureStreamOption {
    pub context: Vec<u8>,
    pub handshake_max_frame_length: usize,
    pub rekey_after_bytes: u64,
    pub rekey_after_records: u64,
}

impl OmniSecureStreamOption {
    pub fn new(context: impl AsRef<[u8]>) -> Result<Self> {
        let value = Self {
            context: context.as_ref().to_vec(),
            handshake_max_frame_length: V2_HANDSHAKE_MAX_FRAME_LENGTH as usize,
            rekey_after_bytes: V2_EPOCH_MAX_PLAINTEXT_LENGTH,
            rekey_after_records: V2_EPOCH_MAX_RECORD_COUNT,
        };
        value.validate()?;
        Ok(value)
    }

    pub(super) fn validate(&self) -> Result<()> {
        if self.context.is_empty()
            || self.context.len() > V2_MAX_CONTEXT_LENGTH as usize
            || self.handshake_max_frame_length == 0
            || self.handshake_max_frame_length > V2_HANDSHAKE_MAX_FRAME_LENGTH as usize
            || !(9..=V2_EPOCH_MAX_PLAINTEXT_LENGTH).contains(&self.rekey_after_bytes)
            || !(2..=V2_EPOCH_MAX_RECORD_COUNT).contains(&self.rekey_after_records)
        {
            return Err(Error::new(ErrorKind::InvalidFormat).with_message("invalid V2 secure stream options"));
        }
        Ok(())
    }
}

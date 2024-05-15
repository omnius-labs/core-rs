use bitflags::bitflags;
use serde::{Deserialize, Serialize};

bitflags! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct OmniSecureStreamVersion: u32 {
        const Unknown = 0;
        const V1 = 1;
    }
}

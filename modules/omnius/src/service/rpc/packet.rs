use serde::{de::DeserializeOwned, Serialize};

#[allow(unused)]
pub enum Packet<TContinueMessage, TCompletedMessage, TErrorMessage>
where
    TContinueMessage: Serialize + DeserializeOwned,
    TCompletedMessage: Serialize + DeserializeOwned,
    TErrorMessage: Serialize + DeserializeOwned,
{
    None,
    Continue { message: TContinueMessage },
    Completed { message: TCompletedMessage },
    Error { message: TErrorMessage },
}

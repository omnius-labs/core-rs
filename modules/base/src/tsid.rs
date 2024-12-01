// Timestamped sortable ID

use std::fmt::Display;

use chrono::{DateTime, Utc};

use crate::{clock::Clock, random_bytes::RandomBytesProvider};

pub trait TsidProvider {
    fn gen(&mut self) -> Tsid;
}

pub struct TsidProviderImpl<TClock, TRandomBytesProvider>
where
    TClock: Clock<Utc>,
    TRandomBytesProvider: RandomBytesProvider,
{
    pub clock: TClock,
    pub random_bytes_provider: TRandomBytesProvider,
    pub random_byte_count: usize,
}

pub struct Tsid {
    pub timestamp: DateTime<Utc>,
    pub random_bytes: Vec<u8>,
}

impl<TSystemClock, TRandomBytesProvider> TsidProviderImpl<TSystemClock, TRandomBytesProvider>
where
    TSystemClock: Clock<Utc>,
    TRandomBytesProvider: RandomBytesProvider,
{
    pub fn new(
        clock: TSystemClock,
        random_bytes_provider: TRandomBytesProvider,
        random_byte_count: usize,
    ) -> Self {
        Self {
            clock,
            random_bytes_provider,
            random_byte_count,
        }
    }
}

impl<TSystemClock, TRandomBytesProvider> TsidProvider
    for TsidProviderImpl<TSystemClock, TRandomBytesProvider>
where
    TSystemClock: Clock<Utc>,
    TRandomBytesProvider: RandomBytesProvider,
{
    fn gen(&mut self) -> Tsid {
        let timestamp = self.clock.now();
        let random_bytes = self.random_bytes_provider.get_bytes(self.random_byte_count);
        Tsid {
            timestamp,
            random_bytes,
        }
    }
}

impl Display for Tsid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let seconds = self.timestamp.timestamp();
        let nanos = self.timestamp.timestamp_subsec_nanos();
        let random_bytes_str = hex::encode(&self.random_bytes);
        write!(f, "{}.{:09}.{}", seconds, nanos, random_bytes_str)
    }
}

#[cfg(test)]
mod tests {
    use crate::{clock::ClockUtc, random_bytes::FakeRandomBytesProvider};

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn print_test() {
        let mut p = TsidProviderImpl::new(ClockUtc, FakeRandomBytesProvider::new(), 16);
        println!("{:}", p.gen());
    }
}

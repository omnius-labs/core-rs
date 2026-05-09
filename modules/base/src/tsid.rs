// Timestamped sortable ID

use std::fmt::Display;

use chrono::{DateTime, Utc};

use crate::clock::Clock;

pub trait TsidProvider {
    fn create(&mut self) -> Tsid;
}

pub struct TsidProviderImpl<TClock, TRng>
where
    TClock: Clock<Utc>,
    TRng: rand::Rng,
{
    clock: TClock,
    rng: TRng,
    random_byte_count: usize,
}

pub struct Tsid {
    pub timestamp: DateTime<Utc>,
    pub random_bytes: Vec<u8>,
}

impl<TSystemClock, TRng> TsidProviderImpl<TSystemClock, TRng>
where
    TSystemClock: Clock<Utc>,
    TRng: rand::Rng,
{
    pub fn new(clock: TSystemClock, rng: TRng, random_byte_count: usize) -> Self {
        Self { clock, rng, random_byte_count }
    }
}

impl<TSystemClock, TRng> TsidProvider for TsidProviderImpl<TSystemClock, TRng>
where
    TSystemClock: Clock<Utc>,
    TRng: rand::Rng,
{
    fn create(&mut self) -> Tsid {
        let timestamp = self.clock.now();
        let mut random_bytes = vec![0; self.random_byte_count];
        self.rng.fill_bytes(&mut random_bytes);
        Tsid { timestamp, random_bytes }
    }
}

impl Display for Tsid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let seconds = self.timestamp.timestamp();
        let nanos = self.timestamp.timestamp_subsec_nanos();
        let random_bytes_str = hex::encode(&self.random_bytes);
        write!(f, "{seconds}.{nanos:09}.{random_bytes_str}")
    }
}

#[cfg(test)]
mod tests {
    use rand::{SeedableRng as _, rngs::ChaCha20Rng};
    use testresult::TestResult;

    use crate::clock::FakeClockUtc;

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn print_test() -> TestResult<()> {
        let clock = FakeClockUtc::new(DateTime::parse_from_rfc3339("2000-01-01T00:00:00Z")?.into());
        let mut tsid_provider = TsidProviderImpl::new(clock, ChaCha20Rng::seed_from_u64(0), 16);
        println!("{:}", tsid_provider.create());

        Ok(())
    }
}

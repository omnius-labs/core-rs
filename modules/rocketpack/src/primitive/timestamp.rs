use chrono::{DateTime, Timelike, Utc};

use crate::{RocketPackDecoder, RocketPackDecoderError, RocketPackEncoder, RocketPackEncoderError, RocketPackStruct};

pub struct Timestamp64 {
    pub seconds: i64,
}

impl Timestamp64 {
    pub fn new(seconds: i64) -> Self {
        Timestamp64 { seconds }
    }

    pub fn to_date_time(&self) -> Option<DateTime<Utc>> {
        DateTime::<Utc>::from_timestamp(self.seconds, 0)
    }
}

impl From<DateTime<Utc>> for Timestamp64 {
    fn from(value: DateTime<Utc>) -> Self {
        Self::new(value.timestamp())
    }
}

impl RocketPackStruct for Timestamp64 {
    fn pack(encoder: &mut impl RocketPackEncoder, value: &Self) -> std::result::Result<(), RocketPackEncoderError> {
        encoder.write_i64(value.seconds)?;

        Ok(())
    }

    fn unpack(decoder: &mut impl RocketPackDecoder) -> std::result::Result<Self, RocketPackDecoderError>
    where
        Self: Sized,
    {
        let value = decoder.read_i64()?;

        Ok(Self::new(value))
    }
}

pub struct Timestamp96 {
    pub seconds: i64,
    pub nanos: u32,
}

impl Timestamp96 {
    pub fn new(seconds: i64, nanos: u32) -> Self {
        Timestamp96 { seconds, nanos }
    }

    pub fn to_date_time(&self) -> Option<DateTime<Utc>> {
        DateTime::<Utc>::from_timestamp(self.seconds, self.nanos)
    }
}

impl From<DateTime<Utc>> for Timestamp96 {
    fn from(value: DateTime<Utc>) -> Self {
        Self::new(value.timestamp(), value.nanosecond())
    }
}

impl RocketPackStruct for Timestamp96 {
    fn pack(encoder: &mut impl RocketPackEncoder, value: &Self) -> std::result::Result<(), RocketPackEncoderError> {
        encoder.write_map(2)?;

        encoder.write_u64(0)?;
        encoder.write_i64(value.seconds)?;

        encoder.write_u64(1)?;
        encoder.write_u32(value.nanos)?;

        Ok(())
    }

    fn unpack(decoder: &mut impl RocketPackDecoder) -> std::result::Result<Self, RocketPackDecoderError>
    where
        Self: Sized,
    {
        let count = decoder.read_map()?;

        let mut seconds: i64 = 0;
        let mut nanos: u32 = 0;

        for _ in 0..count {
            match decoder.read_u64()? {
                0 => seconds = decoder.read_i64()?,
                1 => nanos = decoder.read_u32()?,
                _ => decoder.skip_field()?,
            }
        }

        Ok(Self::new(seconds, nanos))
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use testresult::TestResult;

    use super::*;

    #[test]
    fn normal_timestamp64_test() -> TestResult {
        let example_time: DateTime<Utc> = DateTime::parse_from_rfc3339("2000-01-01T01:01:01Z")?.to_utc();
        let t = Timestamp64::from(example_time);
        let t2 = t.to_date_time().unwrap();
        assert_eq!(example_time, t2);

        Ok(())
    }

    #[test]
    fn normal_timestamp96_test() -> TestResult {
        let example_time: DateTime<Utc> = DateTime::parse_from_rfc3339("2000-01-01T01:01:01.001Z")?.to_utc();
        let t = Timestamp96::from(example_time);
        let t2 = t.to_date_time().unwrap();
        assert_eq!(example_time, t2);

        Ok(())
    }
}

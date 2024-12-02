use chrono::{DateTime, Utc};

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
    fn from(t: DateTime<Utc>) -> Timestamp64 {
        Timestamp64 {
            seconds: t.timestamp(),
        }
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
    fn from(t: DateTime<Utc>) -> Timestamp96 {
        Timestamp96 {
            seconds: t.timestamp(),
            nanos: t.timestamp_subsec_nanos(),
        }
    }
}

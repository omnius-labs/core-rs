use chrono::{DateTime, TimeZone, Utc};

pub trait SystemClock<Tz: TimeZone> {
    fn now(&self) -> DateTime<Tz>;
}

pub struct SystemClockUtc;

impl SystemClock<Utc> for SystemClockUtc {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

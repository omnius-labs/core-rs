use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};
use parking_lot::Mutex;

pub trait Clock<Tz: TimeZone> {
    fn now(&self) -> DateTime<Tz>;
}

pub struct ClockUtc;

impl Clock<Utc> for ClockUtc {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

use tokio::time::Duration;

pub struct FakeClockUtc {
    current_time: Arc<Mutex<DateTime<Utc>>>,
}

impl Clock<Utc> for FakeClockUtc {
    fn now(&self) -> DateTime<Utc> {
        *self.current_time.lock()
    }
}

impl FakeClockUtc {
    pub fn new(start: DateTime<Utc>) -> Self {
        Self {
            current_time: Arc::new(Mutex::new(start)),
        }
    }

    pub fn advance_time(&self, duration: Duration) {
        let mut current_time = self.current_time.lock();
        *current_time += duration;
    }
}

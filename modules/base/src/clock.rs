use std::sync::{Arc, Mutex};

use chrono::{DateTime, TimeZone, Utc};

pub trait Clock<Tz: TimeZone> {
    fn now(&self) -> DateTime<Tz>;
}

pub struct RealClockUtc;

impl Clock<Utc> for RealClockUtc {
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
        *self.current_time.lock().unwrap()
    }
}

impl FakeClockUtc {
    pub fn new(start: DateTime<Utc>) -> Self {
        Self {
            current_time: Arc::new(Mutex::new(start)),
        }
    }

    pub fn advance_time(&self, duration: Duration) {
        let mut current_time = self.current_time.lock().unwrap();
        *current_time += duration;
    }
}

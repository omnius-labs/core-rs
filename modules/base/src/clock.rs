use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

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

pub struct SystemClockUtcMock {
    pub vs: Arc<Mutex<VecDeque<DateTime<Utc>>>>,
}

impl SystemClock<Utc> for SystemClockUtcMock {
    fn now(&self) -> DateTime<Utc> {
        self.vs.lock().unwrap().pop_front().unwrap()
    }
}

impl SystemClockUtcMock {
    pub fn new(vs: Vec<DateTime<Utc>>) -> Self {
        Self {
            vs: Arc::new(Mutex::new(VecDeque::from(vs))),
        }
    }
}

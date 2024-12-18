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

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;

    #[test]
    fn timestamp64_test() {
        let t: DateTime<Utc> = DateTime::parse_from_rfc3339("2000-01-01T00:00:00Z")
            .unwrap()
            .into();

        let ts1 = Timestamp64::new(946684800);
        assert_eq!(ts1.seconds, 946684800);
        assert_eq!(ts1.to_date_time(), Some(t));

        let ts2 = Timestamp64::from(t);
        assert_eq!(ts2.seconds, 946684800);
        assert_eq!(ts2.to_date_time(), Some(t));
    }

    #[test]
    fn timestamp96_test() {
        let t: DateTime<Utc> = DateTime::parse_from_rfc3339("2000-01-01T00:00:00Z")
            .unwrap()
            .into();
        let t = t
            .checked_add_signed(Duration::nanoseconds(123456789))
            .unwrap();

        let ts1 = Timestamp96::new(946684800, 123456789);
        assert_eq!(ts1.seconds, 946684800);
        assert_eq!(ts1.nanos, 123456789);
        assert_eq!(ts1.to_date_time(), Some(t));

        let ts2 = Timestamp96::from(t);
        assert_eq!(ts2.seconds, 946684800);
        assert_eq!(ts2.nanos, 123456789);
        assert_eq!(ts2.to_date_time(), Some(t));
    }
}

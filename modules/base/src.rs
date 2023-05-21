pub trait SystemClock {
    fn utc_now() -> DateTime<Utc>;
}

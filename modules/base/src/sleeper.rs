use std::time::Duration;

use async_trait::async_trait;

#[async_trait]
pub trait Sleeper {
    async fn sleep(&self, duration: Duration);
}

pub struct SleeperImpl;

#[async_trait]
impl Sleeper for SleeperImpl {
    async fn sleep(&self, duration: Duration) {
        tokio::time::sleep(duration).await;
    }
}

pub struct FakeSleeper;

#[async_trait]
impl Sleeper for FakeSleeper {
    async fn sleep(&self, _duration: Duration) {
        tokio::task::yield_now().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_do_something() {
        let fake_sleeper = FakeSleeper;
        fake_sleeper.sleep(Duration::from_secs(1)).await;
    }
}

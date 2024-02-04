use std::{cell::RefCell, collections::VecDeque};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};

use super::S3Client;

pub struct S3ClientMock {
    pub gen_get_presigned_uri_inputs: RefCell<Vec<GenGetPresignedUriInput>>,
    pub gen_put_presigned_uri_inputs: RefCell<Vec<GenPutPresignedUriInput>>,
    pub get_object_inputs: RefCell<Vec<GetObject>>,
    pub put_object_inputs: RefCell<Vec<PutObject>>,

    pub gen_get_presigned_uri_outputs: RefCell<VecDeque<String>>,
    pub gen_put_presigned_uri_outputs: RefCell<VecDeque<String>>,
}

unsafe impl Sync for S3ClientMock {}
unsafe impl Send for S3ClientMock {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenGetPresignedUriInput {
    key: String,
    start_time: DateTime<Utc>,
    expires_in: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenPutPresignedUriInput {
    key: String,
    start_time: DateTime<Utc>,
    expires_in: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetObject {
    key: String,
    destination: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PutObject {
    key: String,
    source: String,
}

#[async_trait]
impl S3Client for S3ClientMock {
    async fn gen_get_presigned_uri(&self, key: &str, start_time: DateTime<Utc>, expires_in: Duration) -> anyhow::Result<String> {
        self.gen_get_presigned_uri_inputs.borrow_mut().push(GenGetPresignedUriInput {
            key: key.to_string(),
            start_time,
            expires_in,
        });

        let output = self.gen_get_presigned_uri_outputs.borrow_mut().pop_front().unwrap_or_default();
        Ok(output)
    }

    async fn gen_put_presigned_uri(&self, key: &str, start_time: DateTime<Utc>, expires_in: Duration) -> anyhow::Result<String> {
        self.gen_put_presigned_uri_inputs.borrow_mut().push(GenPutPresignedUriInput {
            key: key.to_string(),
            start_time,
            expires_in,
        });

        let output = self.gen_put_presigned_uri_outputs.borrow_mut().pop_front().unwrap_or_default();
        Ok(output)
    }

    async fn get_object(&self, key: &str, destination: &str) -> anyhow::Result<()> {
        self.get_object_inputs.borrow_mut().push(GetObject {
            key: key.to_string(),
            destination: destination.to_string(),
        });
        Ok(())
    }

    async fn put_object(&self, key: &str, source: &str) -> anyhow::Result<()> {
        self.put_object_inputs.borrow_mut().push(PutObject {
            key: key.to_string(),
            source: source.to_string(),
        });
        Ok(())
    }
}

impl S3ClientMock {
    #[allow(unused)]
    pub fn new() -> Self {
        Self {
            gen_get_presigned_uri_inputs: RefCell::new(vec![]),
            gen_put_presigned_uri_inputs: RefCell::new(vec![]),
            get_object_inputs: RefCell::new(vec![]),
            put_object_inputs: RefCell::new(vec![]),
            gen_get_presigned_uri_outputs: RefCell::new(VecDeque::new()),
            gen_put_presigned_uri_outputs: RefCell::new(VecDeque::new()),
        }
    }
}

impl Default for S3ClientMock {
    fn default() -> Self {
        Self::new()
    }
}

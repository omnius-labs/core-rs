use std::{collections::VecDeque, sync::Arc};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use parking_lot::Mutex;

use super::S3Client;

pub struct S3ClientMock {
    pub gen_get_presigned_uri_inputs: Arc<Mutex<Vec<GenGetPresignedUriInput>>>,
    pub gen_put_presigned_uri_inputs: Arc<Mutex<Vec<GenPutPresignedUriInput>>>,
    pub get_object_inputs: Arc<Mutex<Vec<GetObject>>>,
    pub put_object_inputs: Arc<Mutex<Vec<PutObject>>>,

    pub gen_get_presigned_uri_outputs: Arc<Mutex<VecDeque<String>>>,
    pub gen_put_presigned_uri_outputs: Arc<Mutex<VecDeque<String>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenGetPresignedUriInput {
    pub key: String,
    pub start_time: DateTime<Utc>,
    pub expires_in: Duration,
    pub filename: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenPutPresignedUriInput {
    pub key: String,
    pub start_time: DateTime<Utc>,
    pub expires_in: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetObject {
    pub key: String,
    pub destination: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PutObject {
    pub key: String,
    pub source: String,
}

#[async_trait]
impl S3Client for S3ClientMock {
    async fn gen_get_presigned_uri(&self, key: &str, start_time: DateTime<Utc>, expires_in: Duration, filename: &str) -> anyhow::Result<String> {
        self.gen_get_presigned_uri_inputs.lock().push(GenGetPresignedUriInput {
            key: key.to_string(),
            start_time,
            expires_in,
            filename: filename.to_string(),
        });

        let output = self.gen_get_presigned_uri_outputs.lock().pop_front().unwrap_or_default();
        Ok(output)
    }

    async fn gen_put_presigned_uri(&self, key: &str, start_time: DateTime<Utc>, expires_in: Duration) -> anyhow::Result<String> {
        self.gen_put_presigned_uri_inputs.lock().push(GenPutPresignedUriInput {
            key: key.to_string(),
            start_time,
            expires_in,
        });

        let output = self.gen_put_presigned_uri_outputs.lock().pop_front().unwrap_or_default();
        Ok(output)
    }

    async fn get_object(&self, key: &str, destination: &str) -> anyhow::Result<()> {
        self.get_object_inputs.lock().push(GetObject {
            key: key.to_string(),
            destination: destination.to_string(),
        });
        Ok(())
    }

    async fn put_object(&self, key: &str, source: &str) -> anyhow::Result<()> {
        self.put_object_inputs.lock().push(PutObject {
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
            gen_get_presigned_uri_inputs: Arc::new(Mutex::new(vec![])),
            gen_put_presigned_uri_inputs: Arc::new(Mutex::new(vec![])),
            get_object_inputs: Arc::new(Mutex::new(vec![])),
            put_object_inputs: Arc::new(Mutex::new(vec![])),
            gen_get_presigned_uri_outputs: Arc::new(Mutex::new(VecDeque::new())),
            gen_put_presigned_uri_outputs: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
}

impl Default for S3ClientMock {
    fn default() -> Self {
        Self::new()
    }
}

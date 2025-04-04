use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use parking_lot::Mutex;

use crate::Result;

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
    pub file_name: String,
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
    pub destination: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PutObject {
    pub key: String,
    pub source: PathBuf,
}

#[async_trait]
impl S3Client for S3ClientMock {
    async fn gen_get_presigned_uri(&self, key: &str, start_time: DateTime<Utc>, expires_in: Duration, file_name: &str) -> Result<String> {
        self.gen_get_presigned_uri_inputs.lock().push(GenGetPresignedUriInput {
            key: key.to_string(),
            start_time,
            expires_in,
            file_name: file_name.to_string(),
        });

        let output = self.gen_get_presigned_uri_outputs.lock().pop_front().unwrap_or_default();
        Ok(output)
    }

    async fn gen_put_presigned_uri(&self, key: &str, start_time: DateTime<Utc>, expires_in: Duration) -> Result<String> {
        self.gen_put_presigned_uri_inputs.lock().push(GenPutPresignedUriInput {
            key: key.to_string(),
            start_time,
            expires_in,
        });

        let output = self.gen_put_presigned_uri_outputs.lock().pop_front().unwrap_or_default();
        Ok(output)
    }

    async fn get_object(&self, key: &str, destination: &Path) -> Result<()> {
        self.get_object_inputs.lock().push(GetObject {
            key: key.to_string(),
            destination: destination.to_path_buf(),
        });
        Ok(())
    }

    async fn put_object(&self, key: &str, source: &Path) -> Result<()> {
        self.put_object_inputs.lock().push(PutObject {
            key: key.to_string(),
            source: source.to_path_buf(),
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

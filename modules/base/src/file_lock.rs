use std::path::{Path, PathBuf};

use tokio::{
    fs::{File, OpenOptions},
    io::AsyncWriteExt,
};

#[allow(unused)]
#[derive(Debug)]
pub struct FileLock {
    path: PathBuf,
    _file: File,
}

impl FileLock {
    #[allow(unused)]
    pub async fn acquire<P: AsRef<Path>>(path: P) -> tokio::io::Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = OpenOptions::new().write(true).create_new(true).open(path).await?;
        file.write_all(std::process::id().to_string().as_bytes()).await?;

        Ok(Self {
            path: path.to_path_buf(),
            _file: file,
        })
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use testresult::TestResult;

    use super::*;

    #[tokio::test]
    async fn acquire_when_create_lock_file() -> TestResult {
        let temp_dir = TempDir::new()?;
        let lock_path = temp_dir.path().join("test.lock");
        let _lock = FileLock::acquire(&lock_path).await?;
        assert!(lock_path.exists());
        let body = std::fs::read_to_string(lock_path)?;
        assert_eq!(body, std::process::id().to_string());

        Ok(())
    }

    #[tokio::test]
    async fn acquire_when_lock_file_exists() -> TestResult {
        let temp_dir = TempDir::new()?;
        let lock_path = temp_dir.path().join("test.lock");
        let _lock = FileLock::acquire(&lock_path).await?;
        let lock_err = FileLock::acquire(&lock_path).await.expect_err("acquire should fail");
        assert_eq!(lock_err.kind(), tokio::io::ErrorKind::AlreadyExists);

        Ok(())
    }

    #[tokio::test]
    async fn drop_release_lock() -> TestResult {
        let temp_dir = TempDir::new()?;
        let lock_path = temp_dir.path().join("test.lock");

        {
            let _lock = FileLock::acquire(&lock_path).await?;
        }

        assert!(!lock_path.exists());

        Ok(())
    }
}

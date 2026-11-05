use std::{
    fs::{File, OpenOptions, TryLockError},
    io::{Seek as _, SeekFrom, Write as _},
    path::Path,
};

/// Holds an OS advisory lock on a file, which the OS releases when the process exits in any way.
/// The file stays in place after the release: deleting it would let one process lock the unlinked file while
/// another locks a new file at the same path.
#[allow(unused)]
#[derive(Debug)]
pub struct FileLock {
    _file: File,
}

impl FileLock {
    /// Returns `AlreadyExists` while another holder has the lock.
    #[allow(unused)]
    pub async fn acquire<P: AsRef<Path>>(path: P) -> tokio::io::Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = OpenOptions::new().read(true).write(true).create(true).truncate(false).open(path)?;
        match file.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return Err(std::io::ErrorKind::AlreadyExists.into()),
            Err(TryLockError::Error(error)) => return Err(error),
        }

        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        file.write_all(std::process::id().to_string().as_bytes())?;
        file.flush()?;

        Ok(Self { _file: file })
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{BufRead as _, BufReader},
        process::{Command, Stdio},
    };

    use tempfile::TempDir;
    use testresult::TestResult;

    use super::*;

    const HOLDER_PATH_ENV: &str = "FILE_LOCK_TEST_HOLDER_PATH";
    const HOLDER_READY: &str = "lock-held";

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
    async fn acquire_when_lock_is_held() -> TestResult {
        let temp_dir = TempDir::new()?;
        let lock_path = temp_dir.path().join("test.lock");
        let _lock = FileLock::acquire(&lock_path).await?;
        let lock_err = FileLock::acquire(&lock_path).await.expect_err("acquire should fail");
        assert_eq!(lock_err.kind(), tokio::io::ErrorKind::AlreadyExists);

        Ok(())
    }

    #[tokio::test]
    async fn drop_release_lock_and_keep_file() -> TestResult {
        let temp_dir = TempDir::new()?;
        let lock_path = temp_dir.path().join("test.lock");

        {
            let _lock = FileLock::acquire(&lock_path).await?;
        }

        assert!(lock_path.exists());
        let _lock = FileLock::acquire(&lock_path).await?;

        Ok(())
    }

    #[tokio::test]
    async fn acquire_replaces_the_process_id_of_a_longer_leftover() -> TestResult {
        let temp_dir = TempDir::new()?;
        let lock_path = temp_dir.path().join("test.lock");
        std::fs::write(&lock_path, "99999999999999999999")?;
        let _lock = FileLock::acquire(&lock_path).await?;
        assert_eq!(std::fs::read_to_string(lock_path)?, std::process::id().to_string());

        Ok(())
    }

    /// The process that `acquire_succeeds_after_the_holder_is_killed` starts and kills. It does nothing on its own.
    #[tokio::test]
    async fn holder_process() -> TestResult {
        let Some(lock_path) = std::env::var_os(HOLDER_PATH_ENV) else {
            return Ok(());
        };
        let _lock = FileLock::acquire(lock_path).await?;
        println!("{HOLDER_READY}");
        tokio::time::sleep(std::time::Duration::from_secs(600)).await;

        Ok(())
    }

    #[tokio::test]
    async fn acquire_succeeds_after_the_holder_is_killed() -> TestResult {
        let temp_dir = TempDir::new()?;
        let lock_path = temp_dir.path().join("test.lock");
        let mut holder = Command::new(std::env::current_exe()?)
            .args(["--exact", "file_lock::tests::holder_process", "--nocapture"])
            .env(HOLDER_PATH_ENV, &lock_path)
            .stdout(Stdio::piped())
            .spawn()?;
        let mut lines = BufReader::new(holder.stdout.take().expect("stdout is piped")).lines();
        assert!(lines.any(|line| line.is_ok_and(|line| line.contains(HOLDER_READY))));

        let lock_err = FileLock::acquire(&lock_path).await.expect_err("acquire should fail while the holder lives");
        assert_eq!(lock_err.kind(), tokio::io::ErrorKind::AlreadyExists);

        holder.kill()?;
        holder.wait()?;
        assert!(lock_path.exists());
        let _lock = FileLock::acquire(&lock_path).await?;

        Ok(())
    }
}

use std::{
    fs::{File, OpenOptions},
    io::{Seek as _, SeekFrom, Write as _},
    path::Path,
};

/// Holds an OS advisory lock on a file, which the OS releases when the process exits in any way.
/// The file stays in place after the release: deleting it would let one process lock the unlinked file while
/// another locks a new file at the same path.
///
/// Dropping the lock unlocks it explicitly instead of only closing the file. The lock belongs to the open file
/// description, and a process started on another thread shares that description until it calls exec, so closing
/// alone can leave the lock held for a moment after the drop returns.
#[allow(unused)]
#[derive(Debug)]
pub struct FileLock {
    file: File,
}

impl FileLock {
    /// Returns `WouldBlock` while another holder has the lock.
    #[allow(unused)]
    pub async fn acquire<P: AsRef<Path>>(path: P) -> tokio::io::Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = OpenOptions::new().read(true).write(true).create(true).truncate(false).open(path)?;
        file.try_lock()?;

        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        file.write_all(std::process::id().to_string().as_bytes())?;
        file.flush()?;

        Ok(Self { file })
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        // The OS releases the lock when the last handle closes, so a failure here only delays the release.
        let _ = self.file.unlock();
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

    /// Kills and reaps the holder on drop, so a panic before the explicit kill cannot leave it behind.
    struct KillOnDrop {
        child: std::process::Child,
    }

    impl Drop for KillOnDrop {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }

    #[tokio::test]
    async fn acquire_when_create_lock_file() -> TestResult {
        let temp_dir = TempDir::new()?;
        let lock_path = temp_dir.path().join("test.lock");
        let lock = FileLock::acquire(&lock_path).await?;
        assert!(lock_path.exists());
        // Windows does not let another handle read a file while it is locked.
        drop(lock);
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
        assert_eq!(lock_err.kind(), tokio::io::ErrorKind::WouldBlock);

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

    /// Starting a process makes the child briefly share every open file description of the parent, even those opened
    /// with close-on-exec, until the child calls exec. Once `drop` returns the lock must be free regardless.
    #[tokio::test]
    async fn drop_releases_lock_while_processes_are_spawned_concurrently() -> TestResult {
        let temp_dir = TempDir::new()?;
        let lock_path = temp_dir.path().join("test.lock");
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

        let spawners: Vec<_> = (0..2)
            .map(|_| {
                let stop = stop.clone();
                std::thread::spawn(move || {
                    let exe = std::env::current_exe().expect("current exe");
                    while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                        // Matches no test, so the child exits immediately.
                        let _ = Command::new(&exe)
                            .args(["--exact", "file_lock::tests::no_such_test"])
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .status();
                    }
                })
            })
            .collect();

        let mut result: tokio::io::Result<()> = Ok(());
        for _ in 0..300 {
            result = async {
                drop(FileLock::acquire(&lock_path).await?);
                drop(FileLock::acquire(&lock_path).await?);
                Ok(())
            }
            .await;
            if result.is_err() {
                break;
            }
        }

        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        for spawner in spawners {
            spawner.join().expect("spawner thread");
        }
        result?;

        Ok(())
    }

    #[tokio::test]
    async fn acquire_replaces_the_process_id_of_a_longer_leftover() -> TestResult {
        let temp_dir = TempDir::new()?;
        let lock_path = temp_dir.path().join("test.lock");
        std::fs::write(&lock_path, "99999999999999999999")?;
        drop(FileLock::acquire(&lock_path).await?);
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
        let mut holder = KillOnDrop {
            child: Command::new(std::env::current_exe()?)
                .args(["--exact", "file_lock::tests::holder_process", "--nocapture"])
                .env(HOLDER_PATH_ENV, &lock_path)
                .stdout(Stdio::piped())
                .spawn()?,
        };
        let mut lines = BufReader::new(holder.child.stdout.take().expect("stdout is piped")).lines();
        assert!(lines.any(|line| line.is_ok_and(|line| line.contains(HOLDER_READY))));

        let lock_err = FileLock::acquire(&lock_path).await.expect_err("acquire should fail while the holder lives");
        assert_eq!(lock_err.kind(), tokio::io::ErrorKind::WouldBlock);

        holder.child.kill()?;
        holder.child.wait()?;
        assert!(lock_path.exists());
        let _lock = FileLock::acquire(&lock_path).await?;

        Ok(())
    }
}

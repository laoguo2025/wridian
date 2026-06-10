use crate::runtime::runtime_root;
use std::collections::hash_map::DefaultHasher;
use std::fs::{self, File, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::{Duration, SystemTime};

const LOCK_WAIT_TIMEOUT: Duration = Duration::from_secs(5);
const LOCK_RETRY_DELAY: Duration = Duration::from_millis(35);
const STALE_LOCK_AFTER: Duration = Duration::from_secs(120);

pub(crate) fn with_file_write_lock<T, F>(
    data_dir: &Path,
    target_path: &Path,
    action: F,
) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String>,
{
    let _guard = WriteLockGuard::acquire(data_dir, target_path, LOCK_WAIT_TIMEOUT)?;
    action()
}

#[derive(Debug)]
struct WriteLockGuard {
    path: PathBuf,
}

impl WriteLockGuard {
    fn acquire(data_dir: &Path, target_path: &Path, timeout: Duration) -> Result<Self, String> {
        let lock_dir = runtime_root(data_dir).join("locks");
        fs::create_dir_all(&lock_dir).map_err(|error| format!("写入锁目录创建失败：{error}"))?;
        let lock_path = lock_dir.join(format!("{}.lock", lock_key(target_path)?));
        let started = SystemTime::now();

        loop {
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(mut file) => {
                    write_lock_metadata(&mut file, target_path)?;
                    return Ok(Self { path: lock_path });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    remove_stale_lock(&lock_path);
                    if started.elapsed().unwrap_or_default() >= timeout {
                        return Err(format!(
                            "文件正在被另一个写入操作使用，请稍后重试：{}",
                            target_path.to_string_lossy()
                        ));
                    }
                    sleep(LOCK_RETRY_DELAY);
                }
                Err(error) => return Err(format!("写入锁创建失败：{error}")),
            }
        }
    }
}

impl Drop for WriteLockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn write_lock_metadata(file: &mut File, target_path: &Path) -> Result<(), String> {
    writeln!(file, "pid={}", std::process::id())
        .map_err(|error| format!("写入锁记录失败：{error}"))?;
    writeln!(file, "target={}", target_path.to_string_lossy())
        .map_err(|error| format!("写入锁记录失败：{error}"))
}

fn remove_stale_lock(lock_path: &Path) {
    let Ok(metadata) = fs::metadata(lock_path) else {
        return;
    };
    let Ok(modified) = metadata.modified() else {
        return;
    };
    if modified.elapsed().unwrap_or_default() > STALE_LOCK_AFTER {
        let _ = fs::remove_file(lock_path);
    }
}

fn lock_key(target_path: &Path) -> Result<String, String> {
    let normalized = normalize_lock_target(target_path)?;
    let mut hasher = DefaultHasher::new();
    normalized.hash(&mut hasher);
    Ok(format!("{:016x}", hasher.finish()))
}

fn normalize_lock_target(target_path: &Path) -> Result<String, String> {
    let absolute = if target_path.is_absolute() {
        target_path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| format!("当前目录读取失败：{error}"))?
            .join(target_path)
    };
    let normalized = match (absolute.parent(), absolute.file_name()) {
        (Some(parent), Some(name)) => parent
            .canonicalize()
            .map(|canonical_parent| canonical_parent.join(name))
            .unwrap_or(absolute),
        _ => absolute,
    };
    Ok(normalized
        .to_string_lossy()
        .replace('\\', "/")
        .to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_data_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "wridian-lock-test-{}-{}",
            name,
            crate::runtime::unique_test_suffix()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp data dir");
        path
    }

    #[test]
    fn write_lock_rejects_second_writer_until_released() {
        let data_dir = temp_data_dir("busy");
        crate::runtime::ensure_workspace(&data_dir).expect("ensure workspace");
        let target = data_dir.join("note.md");
        let guard = WriteLockGuard::acquire(&data_dir, &target, Duration::from_millis(10))
            .expect("first lock");

        let error = WriteLockGuard::acquire(&data_dir, &target, Duration::from_millis(10))
            .expect_err("second lock should fail while first guard is alive");
        assert!(error.contains("另一个写入操作"));

        drop(guard);
        WriteLockGuard::acquire(&data_dir, &target, Duration::from_millis(10))
            .expect("lock after release");
    }
}

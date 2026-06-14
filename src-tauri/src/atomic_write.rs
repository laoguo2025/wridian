//! 原子写入工具：先写临时文件再 rename，避免崩溃/断电留下截断或损坏文件。
//!
//! 临时文件与目标位于同一目录（同卷），`fs::rename` 在 Windows/Linux 上均为原子替换。
//! 写入失败会清理临时文件，保证不残留。

use std::fs;
use std::io::Write;
use std::path::Path;

/// 将 `content` 原子写入 `path`。
///
/// 流程：写 `<path>.tmp` → `flush`+`sync_all` → `rename` 覆盖目标。
/// 任何步骤失败都会尝试删除临时文件后再返回中文错误。
pub(crate) fn atomic_write(path: &Path, content: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or_else(|| {
        format!(
            "目标路径缺少父目录（{}）",
            path.to_string_lossy()
        )
    })?;
    // 临时文件与目标同目录以保证 rename 在同卷上原子完成。
    let temp_name = temporary_name(path);
    let temp_path = parent.join(&temp_name);

    let result = write_temp_and_rename(path, &temp_path, content);
    if result.is_err() {
        // 残留的临时文件对用户毫无意义，失败路径上尽力清理。
        let _ = fs::remove_file(&temp_path);
    }
    result
}

/// `atomic_write` 的字符串便捷封装，匹配现有 `fs::write(path, content)` 调用点。
pub(crate) fn atomic_write_text(path: &Path, content: &str) -> Result<(), String> {
    atomic_write(path, content.as_bytes())
}

/// 以“仅新建”语义原子创建文件：`OpenOptions::create_new(true)` 在打开时即拒绝已存在目标，
/// 消除“先 exists 检查、再写”之间的 TOCTOU 竞态（符号链接替换可导致写到库外）。
///
/// 成功返回 `Ok(true)`；目标已存在返回 `Ok(false)`（由调用方决定如何报错）；
/// 其他 IO 错误返回 `Err`。写完 `flush`+`sync_all` 后落盘。
pub(crate) fn create_new_file(path: &Path, content: &str) -> Result<bool, String> {
    let mut handle = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(handle) => handle,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => return Ok(false),
        Err(error) => {
            return Err(format!(
                "文件创建失败（{}）：{error}",
                path.to_string_lossy()
            ))
        }
    };
    handle.write_all(content.as_bytes()).map_err(|error| {
        format!(
            "文件写入失败（{}）：{error}",
            path.to_string_lossy()
        )
    })?;
    handle.flush().map_err(|error| {
        format!(
            "文件 flush 失败（{}）：{error}",
            path.to_string_lossy()
        )
    })?;
    let _ = handle.sync_all();
    Ok(true)
}

fn temporary_name(path: &Path) -> String {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "wridian-write".to_string());
    format!(".{file_name}.tmp")
}

fn write_temp_and_rename(
    target: &Path,
    temp: &Path,
    content: &[u8],
) -> Result<(), String> {
    {
        // create_new 防止并发写入同一临时文件串台。
        let mut handle = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(temp)
            .map_err(|error| {
                format!(
                    "临时文件创建失败（{}）：{error}",
                    temp.to_string_lossy()
                )
            })?;
        handle.write_all(content).map_err(|error| {
            format!(
                "临时文件写入失败（{}）：{error}",
                temp.to_string_lossy()
            )
        })?;
        // 先落缓冲再落盘，再 rename —— rename 之后才视为完成。
        handle.flush().map_err(|error| {
            format!("临时文件 flush 失败（{}）：{error}", temp.to_string_lossy())
        })?;
        // fsync 尽力而为：某些文件系统/设备不支持也无所谓，不致命。
        let _ = handle.sync_all();
    }
    fs::rename(temp, target).map_err(|error| {
        format!(
            "文件保存失败（{}）：{error}",
            target.to_string_lossy()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "wridian-atomic-write-test-{}-{}",
            name,
            crate::runtime::unique_test_suffix()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    #[test]
    fn atomic_write_creates_new_file() {
        let dir = temp_dir("create");
        let target = dir.join("note.md");

        atomic_write_text(&target, "hello 世界").expect("write");

        let read = fs::read_to_string(&target).expect("read back");
        assert_eq!(read, "hello 世界");
    }

    #[test]
    fn atomic_write_overwrites_existing() {
        let dir = temp_dir("overwrite");
        let target = dir.join("note.md");
        fs::write(&target, "old").expect("seed");

        atomic_write_text(&target, "new").expect("overwrite");

        let read = fs::read_to_string(&target).expect("read back");
        assert_eq!(read, "new");
    }

    #[test]
    fn atomic_write_leaves_no_temp_on_success() {
        let dir = temp_dir("clean");
        let target = dir.join("note.md");
        let temp_name = temporary_name(&target);

        atomic_write_text(&target, "data").expect("write");

        assert!(!dir.join(&temp_name).exists(), "临时文件应被清理");
    }

    #[test]
    fn atomic_write_truncates_large_content() {
        let dir = temp_dir("truncate");
        let target = dir.join("note.md");
        fs::write(&target, "a very long original content that must be replaced").expect("seed");

        atomic_write_text(&target, "短").expect("write");

        let read = fs::read_to_string(&target).expect("read back");
        assert_eq!(read, "短");
    }

    #[test]
    fn atomic_write_handles_cjk_and_newlines() {
        let dir = temp_dir("cjk");
        let target = dir.join("note.md");
        let content = "标题\n\n第一段「对话」。\n第二段——带破折号。";

        atomic_write_text(&target, content).expect("write");

        let read = fs::read_to_string(&target).expect("read back");
        assert_eq!(read, content);
    }

    #[test]
    fn create_new_file_succeeds_when_absent() {
        let dir = temp_dir("create-new");
        let target = dir.join("fresh.md");

        let created = create_new_file(&target, "新内容").expect("create");

        assert!(created, "新建应返回 true");
        let read = fs::read_to_string(&target).expect("read back");
        assert_eq!(read, "新内容");
    }

    #[test]
    fn create_new_file_reports_false_when_present() {
        let dir = temp_dir("create-exists");
        let target = dir.join("exists.md");
        fs::write(&target, "old").expect("seed");

        let created = create_new_file(&target, "new").expect("no io error");

        assert!(!created, "已存在应返回 false 而非报错");
        // 原文件内容必须保持不变（create_new 不应覆盖）。
        let read = fs::read_to_string(&target).expect("read back");
        assert_eq!(read, "old", "已存在文件不应被改动");
    }
}

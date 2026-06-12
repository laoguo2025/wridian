use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn safe_child_path(
    root: &Path,
    path: &Path,
    label: &str,
) -> Result<Option<PathBuf>, String> {
    let canonical_root = root.canonicalize().map_err(|error| {
        format!(
            "{label}根目录解析失败（{}）：{error}",
            root.to_string_lossy()
        )
    })?;
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        format!(
            "{label}路径信息读取失败（{}）：{error}",
            path.to_string_lossy()
        )
    })?;
    if is_symlink_or_reparse(&metadata) {
        return Ok(None);
    }
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("{label}路径解析失败（{}）：{error}", path.to_string_lossy()))?;
    if !canonical.starts_with(&canonical_root) {
        return Ok(None);
    }
    Ok(Some(canonical))
}

pub(crate) fn is_symlink_or_reparse(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink() || is_reparse_point(metadata)
}

#[cfg(windows)]
fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "wridian-path-safety-test-{}-{}",
            name,
            crate::runtime::unique_test_suffix()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    #[test]
    fn safe_child_path_accepts_normal_child() {
        let root = temp_dir("normal");
        let file = root.join("note.md");
        fs::write(&file, "ok").expect("write child");

        let resolved = safe_child_path(&root.canonicalize().unwrap(), &file, "test")
            .expect("safe child")
            .expect("child accepted");

        assert!(resolved.ends_with("note.md"));
    }

    #[test]
    fn safe_child_path_rejects_canonical_path_outside_root() {
        let root = temp_dir("root");
        let outside = temp_dir("outside").join("secret.md");
        fs::write(&outside, "secret").expect("write outside");

        let resolved = safe_child_path(&root.canonicalize().unwrap(), &outside, "test")
            .expect("outside check");

        assert!(resolved.is_none());
    }

    #[test]
    fn safe_child_path_rejects_directory_links_when_available() {
        let root = temp_dir("link-root");
        let outside = temp_dir("link-outside");
        let link = root.join("linked");

        if create_dir_link(&outside, &link).is_err() {
            return;
        }

        let resolved =
            safe_child_path(&root.canonicalize().unwrap(), &link, "test").expect("link check");

        assert!(resolved.is_none());
    }

    #[cfg(windows)]
    fn create_dir_link(target: &Path, link: &Path) -> std::io::Result<()> {
        std::os::windows::fs::symlink_dir(target, link)
    }

    #[cfg(unix)]
    fn create_dir_link(target: &Path, link: &Path) -> std::io::Result<()> {
        std::os::unix::fs::symlink(target, link)
    }
}

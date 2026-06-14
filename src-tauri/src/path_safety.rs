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

/// 校验用户选择的 work/knowledge 根目录是否安全可接受。
///
/// 复用 `canonicalize` + reparse 检测拒绝符号链接根；随后拒绝系统与凭据目录，
/// 避免前端被攻破后把根指向 `C:\Windows`、`~/.ssh` 等（后续读/编辑/回收站都“在根内”
/// 因此被放行）。返回 canonical 路径供调用方使用。
///
/// 注意：这是纵深防御，不是沙箱——用户仍可指向任意普通数据目录。
pub(crate) fn ensure_acceptable_external_root(path: &Path) -> Result<PathBuf, String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        format!("所选目录读取失败（{}）：{error}", path.to_string_lossy())
    })?;
    if is_symlink_or_reparse(&metadata) {
        return Err("不能选择符号链接或重解析点作为根目录。".to_string());
    }
    if !metadata.is_dir() {
        return Err("请选择一个文件夹，而不是文件。".to_string());
    }
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("所选目录解析失败（{}）：{error}", path.to_string_lossy()))?;
    if is_protected_system_root(&canonical) {
        return Err(format!(
            "不能选择系统或凭据目录作为根目录（{}）。请改为选择你自己的作品或资料文件夹。",
            canonical.to_string_lossy()
        ));
    }
    Ok(canonical)
}

/// 判断 canonical 路径是否落在系统/凭据区域。
///
/// 拒绝两类：**完全相等**（盘符根 `C:\`、Unix `/` —— 写作根不应是整盘），
/// 以及**子树**（`C:\Windows` 及其下、`.ssh`/`.aws` 及其下）。
/// 注意盘符根只判等、不判前缀：否则 C 盘任何子目录都会被误拒。
fn is_protected_system_root(canonical: &Path) -> bool {
    let (exact, subtrees) = protected_system_roots();
    if exact.iter().any(|protected| paths_equal(canonical, protected)) {
        return true;
    }
    subtrees.iter().any(|protected| {
        paths_equal(canonical, protected) || is_subpath(canonical, protected)
    })
}

/// 大小写/平台无关的路径相等比较。
fn paths_equal(a: &Path, b: &Path) -> bool {
    components_equal(a.components().collect::<Vec<_>>().as_slice(), b.components().collect::<Vec<_>>().as_slice())
}

/// `path` 是否严格落在 `prefix` 之下（即 `path` 是 `prefix` 的非自身子路径）。
fn is_subpath(path: &Path, prefix: &Path) -> bool {
    let path_components: Vec<_> = path.components().collect();
    let prefix_components: Vec<_> = prefix.components().collect();
    if prefix_components.len() >= path_components.len() {
        return false; // 不是严格的子路径。
    }
    components_equal(&path_components[..prefix_components.len()], &prefix_components)
}

fn components_equal(path: &[std::path::Component], prefix: &[std::path::Component]) -> bool {
    if prefix.len() != path.len() {
        return false;
    }
    prefix.iter().zip(path.iter()).all(|(prefix_comp, path_comp)| {
        match (prefix_comp, path_comp) {
            (std::path::Component::Normal(prefix_name), std::path::Component::Normal(path_name)) => {
                let prefix_str = prefix_name.to_string_lossy();
                let path_str = path_name.to_string_lossy();
                #[cfg(windows)]
                {
                    prefix_str.eq_ignore_ascii_case(&path_str)
                }
                #[cfg(not(windows))]
                {
                    prefix_str == path_str
                }
            }
            (other_prefix, other_path) => other_prefix == other_path,
        }
    })
}

/// 收集当前环境下的受保护系统/凭据目录（canonical 化、失败即跳过）。
///
/// 返回 `(exact, subtrees)`：`exact` 仅判等（盘符根 / Unix 根，拒绝整盘但允许其下子目录）；
/// `subtrees` 判等与子树（系统目录、凭据目录，拒绝其本身及所有子项）。
fn protected_system_roots() -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut exact = Vec::new();
    let mut subtrees = Vec::new();
    #[cfg(windows)]
    {
        if let Ok(system_root) = std::env::var("SystemRoot") {
            push_canonical(&mut subtrees, PathBuf::from(system_root));
        }
        if let Ok(program_files) = std::env::var("ProgramFiles") {
            push_canonical(&mut subtrees, PathBuf::from(program_files));
        }
        if let Ok(program_files_x86) = std::env::var("ProgramFiles(x86)") {
            push_canonical(&mut subtrees, PathBuf::from(program_files_x86));
        }
        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            let profile = PathBuf::from(user_profile);
            push_canonical(&mut subtrees, profile.join(".ssh"));
            push_canonical(&mut subtrees, profile.join(".aws"));
            // 不拒绝整个 AppData：Windows 可能把 Documents 重定向到 AppData 下，
            // 且 Temp 目录也在 AppData\Local\Temp。凭据风险已被 .ssh/.aws 覆盖。
        }
        // 盘符根只判等：拒绝把整盘当根，但不连累盘上任何正常子目录。
        for letter in b'b'..=b'z' {
            let drive_root = PathBuf::from(format!("{}:\\", letter as char));
            if drive_root.exists() {
                push_canonical(&mut exact, drive_root);
            }
        }
    }
    #[cfg(not(windows))]
    {
        // Unix `/` 只判等（拒绝整盘），`/usr` 等判子树。
        push_canonical(&mut exact, PathBuf::from("/"));
        for system_path in ["/usr", "/etc", "/bin", "/sbin", "/boot"] {
            push_canonical(&mut subtrees, PathBuf::from(system_path));
        }
        if let Ok(home) = std::env::var("HOME") {
            let home_path = PathBuf::from(home);
            push_canonical(&mut subtrees, home_path.join(".ssh"));
            push_canonical(&mut subtrees, home_path.join(".aws"));
        }
    }
    (exact, subtrees)
}

fn push_canonical(into: &mut Vec<PathBuf>, path: PathBuf) {
    if let Ok(canonical) = path.canonicalize() {
        into.push(canonical);
    }
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

    #[test]
    fn ensure_acceptable_external_root_accepts_normal_dir() {
        let root = temp_dir("accept-normal");

        let result = ensure_acceptable_external_root(&root);

        let canonical = result.expect("normal dir accepted");
        assert!(
            canonical.ends_with("accept-normal") || canonical.to_string_lossy().contains("accept-normal"),
            "应返回 canonical 路径：{canonical:?}"
        );
    }

    #[test]
    fn ensure_acceptable_external_root_rejects_file() {
        let dir = temp_dir("reject-file");
        let file = dir.join("note.md");
        fs::write(&file, "ok").expect("write file");

        let result = ensure_acceptable_external_root(&file);

        assert!(result.is_err(), "文件不应被接受为根");
    }

    #[test]
    fn ensure_acceptable_external_root_rejects_system_root() {
        #[cfg(windows)]
        let system_root = {
            let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
            std::path::PathBuf::from(system_root)
        };
        #[cfg(not(windows))]
        let system_root = std::path::PathBuf::from("/etc");

        // 系统目录可能不存在于极简 CI，存在时才断言拒绝。
        if system_root.exists() {
            let result = ensure_acceptable_external_root(&system_root);
            assert!(result.is_err(), "系统目录应被拒绝：{system_root:?}");
        }
    }

    #[test]
    fn ensure_acceptable_external_root_rejects_drive_root_on_windows() {
        #[cfg(windows)]
        {
            let drive_root = std::path::PathBuf::from("C:\\");
            if drive_root.exists() {
                let result = ensure_acceptable_external_root(&drive_root);
                assert!(result.is_err(), "盘符根应被拒绝");
            }
        }
        #[cfg(not(windows))]
        {
            let root = std::path::PathBuf::from("/");
            let result = ensure_acceptable_external_root(&root);
            assert!(result.is_err(), "Unix 根目录应被拒绝");
        }
    }
}

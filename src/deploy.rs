use crate::logs;
use crate::models::{Project, Release, AUTH_METHOD_PASSWORD};
use crate::storage::Storage;
use chrono::Local;
use flate2::write::GzEncoder;
use flate2::Compression;
use ssh2::Session;
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;
const SSH_CONNECT_RETRY_COUNT: usize = 3;
const SSH_CONNECT_RETRY_DELAY_MS: u64 = 800;

fn create_session(project: &Project) -> Result<Session, String> {
    let session = create_connected_session(project)?;

    if project.auth_method == AUTH_METHOD_PASSWORD {
        session
            .userauth_password(&project.username, &project.password_or_key)
            .map_err(|e| format!("密码认证失败: {}", e))?;
    } else {
        session
            .userauth_pubkey_file(
                &project.username,
                None,
                Path::new(&project.password_or_key),
                None,
            )
            .map_err(|e| format!("私钥认证失败: {}", e))?;
    }

    if !session.authenticated() {
        return Err("认证失败".to_string());
    }

    Ok(session)
}

fn create_connected_session(project: &Project) -> Result<Session, String> {
    let address = format!("{}:{}", project.remote_host, project.remote_port);
    let mut last_error = String::new();

    for retry_index in 0..=SSH_CONNECT_RETRY_COUNT {
        match connect_ssh_once(&address) {
            Ok(session) => return Ok(session),
            Err(error) => {
                last_error = error;
                if retry_index < SSH_CONNECT_RETRY_COUNT {
                    thread::sleep(Duration::from_millis(SSH_CONNECT_RETRY_DELAY_MS));
                }
            }
        }
    }

    Err(format!(
        "{}（已自动重试 {} 次）",
        last_error, SSH_CONNECT_RETRY_COUNT
    ))
}

fn connect_ssh_once(address: &str) -> Result<Session, String> {
    let tcp = TcpStream::connect(address).map_err(|e| format!("TCP 连接失败: {}", e))?;

    let mut session = Session::new().map_err(|e| format!("创建 SSH 会话失败: {}", e))?;
    session.set_tcp_stream(tcp);
    session
        .handshake()
        .map_err(|e| format!("SSH 握手失败: {}", e))?;

    Ok(session)
}

fn exec_remote(session: &Session, cmd: &str) -> Result<String, String> {
    let mut channel = session
        .channel_session()
        .map_err(|e| format!("创建通道失败: {}", e))?;
    channel
        .exec(cmd)
        .map_err(|e| format!("执行远程命令失败: {}", e))?;

    let mut stdout = String::new();
    channel
        .read_to_string(&mut stdout)
        .map_err(|e| format!("读取输出失败: {}", e))?;

    let mut stderr = String::new();
    channel
        .stderr()
        .read_to_string(&mut stderr)
        .map_err(|e| format!("读取错误输出失败: {}", e))?;

    channel
        .wait_close()
        .map_err(|e| format!("等待通道关闭失败: {}", e))?;

    let exit_status = channel.exit_status().unwrap_or(-1);
    if exit_status != 0 {
        return Err(format!(
            "远程命令退出码 {}: stdout='{}' stderr='{}'",
            exit_status,
            stdout.trim(),
            stderr.trim()
        ));
    }

    Ok(stdout)
}

fn quote_path(path: &str) -> String {
    format!("'{}'", path.replace('\'', "'\\''"))
}

fn is_safe_path(path: &str) -> bool {
    let path = path.trim();
    if path.is_empty()
        || path == "/"
        || path == "/root"
        || path == "/etc"
        || path == "/bin"
        || path == "/sbin"
        || path == "/usr"
    {
        return false;
    }

    !path.contains("..")
}

fn is_valid_timestamp(value: &str) -> bool {
    if value.len() != 15 {
        return false;
    }

    let parts: Vec<&str> = value.split('_').collect();
    if parts.len() != 2 || parts[0].len() != 8 || parts[1].len() != 6 {
        return false;
    }

    parts
        .iter()
        .all(|part| part.chars().all(|ch| ch.is_ascii_digit()))
}

pub fn check_connection(project: &Project) -> Result<String, String> {
    create_session(project).map(|_| "连接成功".to_string())
}

pub fn run_deployment_with_log(
    storage: &Storage,
    project: Project,
    remark: String,
) -> Result<String, String> {
    let project_name = project.name.clone();
    let result = run_deployment(project, remark);

    match &result {
        Ok(message) => {
            let _ = logs::add_log(storage, &project_name, "发布", "成功", message);
        }
        Err(error) => {
            let _ = logs::add_log(storage, &project_name, "发布", "失败", error);
        }
    }

    result
}

fn run_deployment(project: Project, remark: String) -> Result<String, String> {
    if !is_safe_path(&project.remote_deploy_path) {
        return Err(format!(
            "部署路径危险或不合法: {}",
            project.remote_deploy_path
        ));
    }

    let local_path = validate_local_project_path(&project)?;

    if project.uses_build_step() {
        validate_build_command(&project, &local_path)?;
        run_build_command(&project, &local_path)?;
    }

    let output_dir = project.build_output_dir.trim();
    if output_dir.is_empty() {
        return Err("构建产物目录不能为空".to_string());
    }

    let dist_path = local_path.join(output_dir);
    if !dist_path.exists() {
        return Err(format!("构建产物目录不存在: {}", dist_path.display()));
    }
    if !dist_path.is_dir() {
        return Err(format!("构建产物路径不是目录: {}", dist_path.display()));
    }

    let tar_gz_path = std::env::temp_dir().join(format!("deploy_{}.tar.gz", project.id));
    build_archive(&dist_path, &tar_gz_path)?;

    let session = create_session(&project)?;
    ensure_remote_symlink_path_available(&session, &project)?;

    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let release_dir = format!("{}/releases/{}", project.remote_deploy_path, timestamp);
    let mut remote_dir_created = false;

    let result = (|| -> Result<String, String> {
        let release_dir_q = quote_path(&release_dir);
        exec_remote(&session, &format!("mkdir -p {}", release_dir_q))?;
        remote_dir_created = true;

        upload_archive(
            &session,
            &tar_gz_path,
            &format!("{}/deploy.tar.gz", release_dir),
        )?;
        exec_remote(
            &session,
            &format!(
                "cd {} && tar -xzf deploy.tar.gz && rm deploy.tar.gz",
                release_dir_q
            ),
        )?;

        if !remark.trim().is_empty() {
            write_release_note(
                &session,
                &format!("{}/RELEASE_NOTE.txt", release_dir),
                remark.trim(),
            )?;
        }

        let current_link = format!(
            "{}/{}",
            project.remote_deploy_path,
            project.effective_symlink_name()
        );
        exec_remote(
            &session,
            &format!("ln -sfn {} {}", release_dir_q, quote_path(&current_link)),
        )?;

        cleanup_old_releases(&session, &project)?;

        Ok(release_dir.clone())
    })();

    let _ = std::fs::remove_file(&tar_gz_path);

    match result {
        Ok(dir) => Ok(format!("部署成功: {}", dir)),
        Err(error) => {
            if remote_dir_created {
                let _ = exec_remote(&session, &format!("rm -rf {}", quote_path(&release_dir)));
            }
            Err(error)
        }
    }
}

fn validate_local_project_path(project: &Project) -> Result<&Path, String> {
    let local_path_text = project.local_path.trim();
    if local_path_text.is_empty() {
        return Err("本地项目根目录不能为空".to_string());
    }

    let local_path = Path::new(local_path_text);
    if !local_path.exists() {
        return Err(format!("本地项目根目录不存在: {}", local_path.display()));
    }
    if !local_path.is_dir() {
        return Err(format!("本地项目根路径不是目录: {}", local_path.display()));
    }
    Ok(local_path)
}

fn validate_build_command(project: &Project, local_path: &Path) -> Result<(), String> {
    let build_command = project.build_command.trim();
    if build_command.is_empty() {
        return Err("构建命令不能为空".to_string());
    }

    if let Some(script_name) = package_manager_script_name(build_command) {
        validate_package_script(local_path, &script_name, build_command)?;
    }

    Ok(())
}

fn validate_package_script(
    local_path: &Path,
    script_name: &str,
    build_command: &str,
) -> Result<(), String> {
    let package_json_path = local_path.join("package.json");
    if !package_json_path.exists() {
        return Err(format!(
            "本地项目根目录缺少 package.json，无法执行构建命令: {}",
            build_command
        ));
    }

    let content = std::fs::read_to_string(&package_json_path)
        .map_err(|e| format!("读取 package.json 失败: {}", e))?;
    let package_json: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("解析 package.json 失败: {}", e))?;
    let has_script = package_json
        .get("scripts")
        .and_then(|scripts| scripts.as_object())
        .is_some_and(|scripts| scripts.contains_key(script_name));

    if !has_script {
        return Err(format!(
            "本地项目根目录中没有构建命令 \"{}\"，请检查 package.json scripts 或修改构建命令: {}",
            script_name, build_command
        ));
    }

    Ok(())
}

fn package_manager_script_name(command: &str) -> Option<String> {
    let words = split_command_words(command);
    let tool = command_tool_name(words.first()?)?;
    let second = words.get(1)?.as_str();

    match tool.as_str() {
        "npm" => {
            if matches!(second, "run" | "run-script") {
                script_word(&words, 2)
            } else {
                None
            }
        }
        "pnpm" => {
            if second == "run" {
                script_word(&words, 2)
            } else if is_direct_package_script(second, &PNPM_COMMANDS) {
                Some(second.to_string())
            } else {
                None
            }
        }
        "yarn" => {
            if second == "run" {
                script_word(&words, 2)
            } else if is_direct_package_script(second, &YARN_COMMANDS) {
                Some(second.to_string())
            } else {
                None
            }
        }
        "bun" => {
            if second == "run" {
                script_word(&words, 2)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn command_tool_name(command_word: &str) -> Option<String> {
    Path::new(command_word)
        .file_stem()
        .map(|name| name.to_string_lossy().to_ascii_lowercase())
}

fn script_word(words: &[String], index: usize) -> Option<String> {
    words
        .get(index)
        .filter(|word| !word.starts_with('-'))
        .map(|word| word.to_string())
}

fn is_direct_package_script(command: &str, builtins: &[&str]) -> bool {
    !command.starts_with('-') && !builtins.contains(&command)
}

fn split_command_words(command: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote = None;

    for ch in command.chars() {
        match quote {
            Some(quote_ch) if ch == quote_ch => {
                quote = None;
            }
            Some(_) => current.push(ch),
            None if ch == '"' || ch == '\'' => {
                quote = Some(ch);
            }
            None if ch.is_whitespace() => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            None => current.push(ch),
        }
    }

    if !current.is_empty() {
        words.push(current);
    }

    words
}

const PNPM_COMMANDS: [&str; 23] = [
    "add",
    "approve-builds",
    "audit",
    "config",
    "create",
    "dedupe",
    "deploy",
    "exec",
    "fetch",
    "help",
    "import",
    "init",
    "install",
    "licenses",
    "link",
    "list",
    "outdated",
    "patch",
    "prune",
    "publish",
    "rebuild",
    "remove",
    "update",
];

const YARN_COMMANDS: [&str; 23] = [
    "add",
    "bin",
    "cache",
    "config",
    "constraints",
    "dedupe",
    "dlx",
    "exec",
    "explain",
    "help",
    "import",
    "info",
    "init",
    "install",
    "link",
    "node",
    "npm",
    "pack",
    "patch",
    "plugin",
    "remove",
    "set",
    "up",
];

fn run_build_command(project: &Project, local_path: &Path) -> Result<(), String> {
    let (shell, arg) = if cfg!(target_os = "windows") {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };

    let mut command = Command::new(shell);
    command
        .arg(arg)
        .arg(project.build_command.trim())
        .current_dir(local_path);

    #[cfg(target_os = "windows")]
    command.creation_flags(CREATE_NO_WINDOW);

    let output = command
        .output()
        .map_err(|e| format!("执行构建命令失败: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let details = if !stderr.trim().is_empty() {
            stderr.trim()
        } else if !stdout.trim().is_empty() {
            stdout.trim()
        } else {
            "构建命令返回非零退出码"
        };
        return Err(format!("构建失败: {}", details));
    }

    Ok(())
}

fn build_archive(dist_path: &Path, tar_gz_path: &Path) -> Result<(), String> {
    let tar_gz = File::create(tar_gz_path).map_err(|e| format!("创建压缩包失败: {}", e))?;
    let enc = GzEncoder::new(tar_gz, Compression::default());
    let mut builder = tar::Builder::new(enc);
    builder
        .append_dir_all(".", dist_path)
        .map_err(|e| format!("添加文件到压缩包失败: {}", e))?;
    let gz_encoder = builder
        .into_inner()
        .map_err(|e| format!("完成压缩包失败: {}", e))?;
    gz_encoder
        .finish()
        .map_err(|e| format!("刷新压缩包失败: {}", e))?;
    Ok(())
}

fn upload_archive(session: &Session, local_path: &Path, remote_path: &str) -> Result<(), String> {
    let sftp = session
        .sftp()
        .map_err(|e| format!("SFTP 连接失败: {}", e))?;
    let mut remote_file = sftp
        .create(Path::new(remote_path))
        .map_err(|e| format!("创建远程文件失败: {}", e))?;

    let local_data = std::fs::read(local_path).map_err(|e| format!("读取本地压缩包失败: {}", e))?;
    remote_file
        .write_all(&local_data)
        .map_err(|e| format!("上传文件失败: {}", e))?;
    Ok(())
}

fn write_release_note(session: &Session, remote_path: &str, remark: &str) -> Result<(), String> {
    let sftp = session
        .sftp()
        .map_err(|e| format!("SFTP 连接失败: {}", e))?;
    let mut note_file = sftp
        .create(Path::new(remote_path))
        .map_err(|e| format!("创建备注文件失败: {}", e))?;
    note_file
        .write_all(remark.as_bytes())
        .map_err(|e| format!("写入备注失败: {}", e))
}

fn cleanup_old_releases(session: &Session, project: &Project) -> Result<(), String> {
    let releases_path = format!("{}/releases", project.remote_deploy_path);
    let stdout = exec_remote(
        session,
        &format!("ls -1 {} 2>/dev/null", quote_path(&releases_path)),
    )
    .unwrap_or_default();

    let mut releases: Vec<String> = stdout
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| is_valid_timestamp(line))
        .collect();

    releases.sort();

    if releases.len() > project.max_releases as usize {
        let delete_count = releases.len() - project.max_releases as usize;
        for release in &releases[..delete_count] {
            let delete_path = format!("{}/{}", releases_path, release);
            let _ = exec_remote(session, &format!("rm -rf {}", quote_path(&delete_path)));
        }
    }

    Ok(())
}

fn ensure_remote_symlink_path_available(
    session: &Session,
    project: &Project,
) -> Result<(), String> {
    let symlink_path = format!(
        "{}/{}",
        project.remote_deploy_path,
        project.effective_symlink_name()
    );
    let symlink_path_q = quote_path(&symlink_path);
    let stdout = exec_remote(
        session,
        &format!(
            "if [ -e {path} ] && [ ! -L {path} ] && [ -d {path} ]; then printf directory; fi",
            path = symlink_path_q
        ),
    )?;

    if stdout.trim() == "directory" {
        return Err(format!(
            "远程服务器上已存在与软链接名称相同的目录: {} - 请删除或重命名该目录，或修改项目的软链接名称后再发布",
            symlink_path
        ));
    }

    Ok(())
}

pub fn get_releases(project: &Project) -> Result<Vec<Release>, String> {
    if !is_safe_path(&project.remote_deploy_path) {
        return Err("不合法的部署路径".to_string());
    }

    let session = create_session(project)?;
    let current_link_path = format!(
        "{}/{}",
        project.remote_deploy_path,
        project.effective_symlink_name()
    );
    let current_release_full_path = exec_remote(
        &session,
        &format!("readlink -f {} 2>/dev/null", quote_path(&current_link_path)),
    )
    .unwrap_or_default();
    let current_version_name = Path::new(current_release_full_path.trim())
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_string();

    let releases_path = format!("{}/releases", project.remote_deploy_path);
    let stdout = exec_remote(
        &session,
        &format!(
            "ls -1 {} 2>/dev/null || echo ''",
            quote_path(&releases_path)
        ),
    )?;

    let mut releases = Vec::new();
    for release in stdout
        .lines()
        .map(str::trim)
        .filter(|line| is_valid_timestamp(line))
    {
        let note_path = format!(
            "{}/releases/{}/RELEASE_NOTE.txt",
            project.remote_deploy_path, release
        );
        let remark = exec_remote(
            &session,
            &format!("cat {} 2>/dev/null || echo ''", quote_path(&note_path)),
        )
        .unwrap_or_default()
        .trim()
        .to_string();

        releases.push(Release {
            name: release.to_string(),
            remark,
            is_current: release == current_version_name,
        });
    }

    releases.sort_by(|a, b| b.name.cmp(&a.name));
    Ok(releases)
}

pub fn rollback_with_log(
    storage: &Storage,
    project: Project,
    release: String,
) -> Result<String, String> {
    let project_name = project.name.clone();
    let result = rollback(project, release);

    match &result {
        Ok(message) => {
            let _ = logs::add_log(storage, &project_name, "回退", "成功", message);
        }
        Err(error) => {
            let _ = logs::add_log(storage, &project_name, "回退", "失败", error);
        }
    }

    result
}

fn rollback(project: Project, release: String) -> Result<String, String> {
    if !is_safe_path(&project.remote_deploy_path) {
        return Err("不合法的部署路径".to_string());
    }

    if !is_valid_timestamp(&release) {
        return Err(format!("回退目标版本非法: {}", release));
    }

    let session = create_session(&project)?;
    let current_link = format!(
        "{}/{}",
        project.remote_deploy_path,
        project.effective_symlink_name()
    );
    let target_dir = format!("{}/releases/{}", project.remote_deploy_path, release);
    ensure_remote_symlink_path_available(&session, &project)?;

    exec_remote(
        &session,
        &format!(
            "ln -sfn {} {}",
            quote_path(&target_dir),
            quote_path(&current_link)
        ),
    )?;

    Ok(format!("回滚成功: {}", release))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use uuid::Uuid;

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("deploydesk-test-{}", Uuid::new_v4()));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn detects_package_manager_script_commands() {
        assert_eq!(
            package_manager_script_name("npm run build").as_deref(),
            Some("build")
        );
        assert_eq!(
            package_manager_script_name("pnpm build --filter app").as_deref(),
            Some("build")
        );
        assert_eq!(
            package_manager_script_name("yarn \"build:web\"").as_deref(),
            Some("build:web")
        );
        assert_eq!(package_manager_script_name("cargo build"), None);
    }

    #[test]
    fn rejects_missing_local_project_directory() {
        let mut project = Project::blank();
        project.local_path = std::env::temp_dir()
            .join(format!("deploydesk-missing-{}", Uuid::new_v4()))
            .display()
            .to_string();

        let error = validate_local_project_path(&project).unwrap_err();
        assert!(error.contains("本地项目根目录不存在"));
    }

    #[test]
    fn rejects_empty_output_directory_before_remote_connect() {
        let dir = TestDir::new();
        let mut project = Project::blank();
        project.local_path = dir.path.display().to_string();
        project.deploy_mode = crate::models::DEPLOY_MODE_UPLOAD.to_string();
        project.build_output_dir = " ".to_string();

        let error = run_deployment(project, String::new()).unwrap_err();
        assert!(error.contains("构建产物目录不能为空"));
    }

    #[test]
    fn rejects_missing_package_script() {
        let dir = TestDir::new();
        fs::write(
            dir.path.join("package.json"),
            r#"{"scripts":{"dev":"vite --host 0.0.0.0"}}"#,
        )
        .unwrap();

        let error = validate_package_script(&dir.path, "build", "npm run build").unwrap_err();
        assert!(error.contains("本地项目根目录中没有构建命令"));
    }

    #[test]
    fn accepts_existing_package_script() {
        let dir = TestDir::new();
        fs::write(
            dir.path.join("package.json"),
            r#"{"scripts":{"build":"vite build"}}"#,
        )
        .unwrap();

        assert!(validate_package_script(&dir.path, "build", "npm run build").is_ok());
    }
}

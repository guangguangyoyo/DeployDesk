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
use std::path::Path;
use std::process::Command;

fn create_session(project: &Project) -> Result<Session, String> {
    let tcp = TcpStream::connect(format!("{}:{}", project.remote_host, project.remote_port))
        .map_err(|e| format!("TCP 连接失败: {}", e))?;

    let mut session = Session::new().map_err(|e| format!("创建 SSH 会话失败: {}", e))?;
    session.set_tcp_stream(tcp);
    session
        .handshake()
        .map_err(|e| format!("SSH 握手失败: {}", e))?;

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

    if project.uses_build_step() {
        run_build_command(&project)?;
    }

    let dist_path = Path::new(&project.local_path).join(&project.build_output_dir);
    if !dist_path.exists() {
        return Err(format!("构建产物目录不存在: {}", dist_path.display()));
    }

    let tar_gz_path = std::env::temp_dir().join(format!("deploy_{}.tar.gz", project.id));
    build_archive(&dist_path, &tar_gz_path)?;

    let session = create_session(&project)?;
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

fn run_build_command(project: &Project) -> Result<(), String> {
    let (shell, arg) = if cfg!(target_os = "windows") {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };

    let output = Command::new(shell)
        .arg(arg)
        .arg(&project.build_command)
        .current_dir(&project.local_path)
        .output()
        .map_err(|e| format!("执行构建命令失败: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "构建失败: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
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

use crate::projects::Project;
use crate::logs;
use serde::{Deserialize, Serialize};
use chrono::Local;
use flate2::write::GzEncoder;
use flate2::Compression;
use ssh2::Session;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::net::TcpStream;
use std::path::Path;
use std::process::Command;

/// Helper: Create an authenticated SSH session
fn create_session(project: &Project) -> Result<Session, String> {
    let tcp = TcpStream::connect(format!("{}:{}", project.remote_host, project.remote_port))
        .map_err(|e| format!("TCP 连接失败: {}", e))?;
    
    let mut sess = Session::new().map_err(|e| format!("创建 SSH 会话失败: {}", e))?;
    sess.set_tcp_stream(tcp);
    sess.handshake().map_err(|e| format!("SSH 握手失败: {}", e))?;

    if project.auth_method == "password" {
        sess.userauth_password(&project.username, &project.password_or_key)
            .map_err(|e| format!("密码认证失败: {}", e))?;
    } else {
        sess.userauth_pubkey_file(
            &project.username,
            None,
            Path::new(&project.password_or_key),
            None,
        ).map_err(|e| format!("私钥认证失败: {}", e))?;
    }

    if !sess.authenticated() {
        return Err("认证失败".to_string());
    }

    Ok(sess)
}

/// Helper: Execute a remote command and return its stdout + stderr
fn exec_remote(sess: &Session, cmd: &str) -> Result<String, String> {
    let mut channel = sess.channel_session().map_err(|e| format!("创建通道失败: {}", e))?;
    channel.exec(cmd).map_err(|e| format!("执行远程命令失败: {}", e))?;
    
    let mut stdout = String::new();
    channel.read_to_string(&mut stdout).map_err(|e| format!("读取输出失败: {}", e))?;
    
    let mut stderr = String::new();
    channel.stderr().read_to_string(&mut stderr).map_err(|e| format!("读取错误输出失败: {}", e))?;
    
    channel.wait_close().map_err(|e| format!("等待通道关闭失败: {}", e))?;
    
    let exit_status = channel.exit_status().unwrap_or(-1);
    if exit_status != 0 {
        return Err(format!("远程命令退出码 {}: stdout='{}' stderr='{}'", exit_status, stdout.trim(), stderr.trim()));
    }
    
    Ok(stdout)
}

/// Helper: Wrap path in single quotes to prevent injection
fn quote_path(path: &str) -> String {
    format!("'{}'", path.replace("'", "'\\''"))
}

/// Helper: Basic path safety check
fn is_safe_path(path: &str) -> bool {
    let p = path.trim();
    if p.is_empty() || p == "/" || p == "/root" || p == "/etc" || p == "/bin" || p == "/sbin" || p == "/usr" {
        return false;
    }
    // Prevent relative paths that could escape
    if p.contains("..") {
        return false;
    }
    true
}

/// Helper: Validate timestamp format %Y%m%d_%H%M%S (15 chars, e.g., 20240309_110436)
fn is_valid_timestamp(s: &str) -> bool {
    if s.len() != 15 { return false; }
    let parts: Vec<&str> = s.split('_').collect();
    if parts.len() != 2 { return false; }
    if parts[0].len() != 8 || parts[1].len() != 6 { return false; }
    parts[0].chars().all(|c| c.is_ascii_digit()) && parts[1].chars().all(|c| c.is_ascii_digit())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Release {
    pub name: String,
    pub remark: String,
    pub is_current: bool,
}

#[tauri::command]
pub async fn check_connection(project: Project) -> Result<String, String> {
    match create_session(&project) {
        Ok(_) => Ok("连接成功".to_string()),
        Err(e) => Err(format!("连接失败: {}", e)),
    }
}

#[tauri::command]
pub async fn run_deployment(app: tauri::AppHandle, project: Project, remark: String) -> Result<String, String> {
    let project_name = project.name.clone();
    let res = run_deployment_internal(project, remark).await;
    match &res {
        Ok(msg) => {
            let _ = logs::add_log(&app, &project_name, "发布", "成功", msg);
        },
        Err(e) => {
            let _ = logs::add_log(&app, &project_name, "发布", "失败", e);
        }
    }
    res
}

async fn run_deployment_internal(project: Project, remark: String) -> Result<String, String> {
    // 0. 安全控制：校验路径是否合法
    if !is_safe_path(&project.remote_deploy_path) {
        return Err(format!("部署路径危险或不合法: {}", project.remote_deploy_path));
    }

    // 1. 打包 (根据部署模式判断)
    if project.deploy_mode == "build" {
        let (shell, arg) = if cfg!(target_os = "windows") {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };

        let build_output = Command::new(shell)
            .arg(arg)
            .arg(&project.build_command)
            .current_dir(&project.local_path)
            .output()
            .map_err(|e| format!("执行构建命令失败: {}", e))?;

        if !build_output.status.success() {
            return Err(format!("构建失败: {}", String::from_utf8_lossy(&build_output.stderr)));
        }
    }

    // 2. 本地压缩构建产物
    let dist_path = Path::new(&project.local_path).join(&project.build_output_dir);
    if !dist_path.exists() {
        return Err(format!("构建产物目录不存在: {}", dist_path.display()));
    }

    let tar_gz_path = std::env::temp_dir().join(format!("deploy_{}.tar.gz", project.id));
    let tar_gz = File::create(&tar_gz_path).map_err(|e| format!("创建压缩包失败: {}", e))?;
    let enc = GzEncoder::new(tar_gz, Compression::default());
    let mut builder = tar::Builder::new(enc);
    builder.append_dir_all(".", &dist_path).map_err(|e| format!("添加文件到压缩包失败: {}", e))?;
    let gz_encoder = builder.into_inner().map_err(|e| format!("完成压缩包失败: {}", e))?;
    gz_encoder.finish().map_err(|e| format!("刷新压缩包失败: {}", e))?;

    // 3. 建立 SSH 连接
    let sess = create_session(&project)?;

    // 4. 创建远程目录
    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let release_dir = format!("{}/releases/{}", project.remote_deploy_path, timestamp);
    
    // 记录是否创建了远程目录，以便失败时清理
    let mut remote_dir_created = false;

    let res = (|| -> Result<String, String> {
        let release_dir_q = quote_path(&release_dir);
        exec_remote(&sess, &format!("mkdir -p {}", release_dir_q))?;
        remote_dir_created = true;

        // 5. SFTP 上传压缩包
        {
            let sftp = sess.sftp().map_err(|e| format!("SFTP 连接失败: {}", e))?;
            let remote_tar_path = format!("{}/deploy.tar.gz", release_dir);
            
            let mut remote_file = sftp.create(Path::new(&remote_tar_path))
                .map_err(|e| format!("创建远程文件失败: {}", e))?;
            
            let local_data = std::fs::read(&tar_gz_path)
                .map_err(|e| format!("读取本地压缩包失败: {}", e))?;
            
            remote_file.write_all(&local_data)
                .map_err(|e| format!("上传文件失败: {}", e))?;
            
            drop(remote_file);
            drop(sftp);
        }

        // 6. 在服务器上解压
        exec_remote(&sess, &format!("cd {} && tar -xzf deploy.tar.gz && rm deploy.tar.gz", release_dir_q))?;

        // 6.5 保存发布备注
        if !remark.trim().is_empty() {
            let sftp = sess.sftp().map_err(|e| format!("SFTP 连接失败: {}", e))?;
            let note_path = format!("{}/RELEASE_NOTE.txt", release_dir);
            let mut note_file = sftp.create(Path::new(&note_path))
                .map_err(|e| format!("创建备注文件失败: {}", e))?;
            note_file.write_all(remark.trim().as_bytes())
                .map_err(|e| format!("写入备注失败: {}", e))?;
            drop(note_file);
            drop(sftp);
        }

        // 7. 更新软链接
        let symlink = if project.symlink_name.is_empty() { "current".to_string() } else { project.symlink_name.clone() };
        let current_link = format!("{}/{}", project.remote_deploy_path, symlink);
        exec_remote(&sess, &format!("ln -sfn {} {}", release_dir_q, quote_path(&current_link)))?;

        // 8. 清理旧版本
        let releases_path = format!("{}/releases", project.remote_deploy_path);
        let stdout = exec_remote(&sess, &format!("ls -1 {} 2>/dev/null", quote_path(&releases_path))).unwrap_or_default();
        let mut all_releases: Vec<String> = stdout.lines()
            .map(|l| l.trim().to_string())
            .filter(|l| is_valid_timestamp(l))
            .collect();
        
        all_releases.sort();

        if all_releases.len() > project.max_releases as usize {
            let num_to_delete = all_releases.len() - project.max_releases as usize;
            let to_delete = &all_releases[..num_to_delete];
            for rel in to_delete {
                let delete_path = format!("{}/{}", releases_path, rel);
                let _ = exec_remote(&sess, &format!("rm -rf {}", quote_path(&delete_path)));
            }
        }

        Ok(release_dir.clone())
    })();

    // 清理本地临时文件
    let _ = std::fs::remove_file(tar_gz_path);

    match res {
        Ok(dir) => Ok(format!("部署成功: {}", dir)),
        Err(e) => {
            // 如果创建过远程目录但之后失败了，尝试清理
            if remote_dir_created {
                let _ = exec_remote(&sess, &format!("rm -rf {}", quote_path(&release_dir)));
            }
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn get_releases(project: Project) -> Result<Vec<Release>, String> {
    if !is_safe_path(&project.remote_deploy_path) {
        return Err("不合法的部署路径".to_string());
    }
    let sess = create_session(&project)?;

    // 1. 获取当前软链接指向的版本
    let symlink = if project.symlink_name.is_empty() { "current".to_string() } else { project.symlink_name.clone() };
    let current_link_path = format!("{}/{}", project.remote_deploy_path, symlink);
    // 使用 readlink -f 获取绝对路径，如果没有则返回空
    let current_release_full_path = exec_remote(&sess, &format!("readlink -f {} 2>/dev/null", quote_path(&current_link_path))).unwrap_or_default();
    let current_version_name = Path::new(current_release_full_path.trim())
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    // 2. 获取所有版本
    let releases_path = format!("{}/releases", project.remote_deploy_path);
    let stdout = exec_remote(&sess, &format!("ls -1 {} 2>/dev/null || echo ''", quote_path(&releases_path)))?;
    let releases: Vec<String> = stdout.lines()
        .map(|l| l.trim().to_string())
        .filter(|l| is_valid_timestamp(l))
        .collect();
    
    // 3. 读取每个 release 的备注并判断是否为当前版本
    let mut result: Vec<Release> = Vec::new();
    for r in &releases {
        let note_path = format!("{}/releases/{}/RELEASE_NOTE.txt", project.remote_deploy_path, r);
        let note = exec_remote(&sess, &format!("cat {} 2>/dev/null || echo ''", quote_path(&note_path))).unwrap_or_default().trim().to_string();
        
        result.push(Release {
            name: r.clone(),
            remark: note,
            is_current: r == &current_version_name,
        });
    }
    
    Ok(result)
}

#[tauri::command]
pub async fn rollback(app: tauri::AppHandle, project: Project, release: String) -> Result<String, String> {
    let project_name = project.name.clone();
    let res = rollback_internal(project, release).await;
    match &res {
        Ok(msg) => {
            let _ = logs::add_log(&app, &project_name, "回退", "成功", msg);
        },
        Err(e) => {
            let _ = logs::add_log(&app, &project_name, "回退", "失败", e);
        }
    }
    res
}

async fn rollback_internal(project: Project, release: String) -> Result<String, String> {
    if !is_safe_path(&project.remote_deploy_path) {
        return Err("不合法的部署路径".to_string());
    }
    // 额外校验：回退的目标版本必须符合时间戳格式
    if !is_valid_timestamp(&release) {
        return Err(format!("回退目标版本非法: {}", release));
    }

    let sess = create_session(&project)?;

    let symlink = if project.symlink_name.is_empty() { "current".to_string() } else { project.symlink_name.clone() };
    let current_link = format!("{}/{}", project.remote_deploy_path, symlink);
    let target_dir = format!("{}/releases/{}", project.remote_deploy_path, release);
    
    // 执行原子连接更新
    exec_remote(&sess, &format!("ln -sfn {} {}", quote_path(&target_dir), quote_path(&current_link)))?;

    Ok(format!("回滚成功: {}", release))
}

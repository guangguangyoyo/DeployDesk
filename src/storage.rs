use crate::models::{LogEntry, Project, AUTH_METHOD_PASSWORD};
use std::fs;
use std::path::{Path, PathBuf};

const LOCAL_SECRET_PREFIX: &str = "deploydesk-local-secret-v1";
const LOCAL_SECRET_PREFIX_WITH_SEPARATOR: &str = "deploydesk-local-secret-v1:";

#[derive(Clone)]
pub struct Storage {
    app_dir: PathBuf,
}

impl Storage {
    pub fn new() -> Result<Self, String> {
        let app_dir = app_config_dir()?;
        fs::create_dir_all(&app_dir)
            .map_err(|e| format!("创建配置目录失败 {}: {}", app_dir.display(), e))?;
        Ok(Self { app_dir })
    }

    pub fn projects_path(&self) -> PathBuf {
        self.app_dir.join("projects.json")
    }

    pub fn logs_path(&self) -> PathBuf {
        self.app_dir.join("logs.json")
    }

    pub fn save_imported_key(
        &self,
        project_id: &str,
        file_name: &str,
        content: &str,
    ) -> Result<PathBuf, String> {
        let key_dir = self
            .app_dir
            .join("imported-keys")
            .join(safe_path_component(project_id, "project"));
        fs::create_dir_all(&key_dir)
            .map_err(|e| format!("创建导入私钥目录失败 {}: {}", key_dir.display(), e))?;

        let safe_file_name = Path::new(file_name)
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| safe_path_component(name, "identity.key"))
            .unwrap_or_else(|| "identity.key".to_string());
        let key_path = key_dir.join(safe_file_name);
        fs::write(&key_path, content)
            .map_err(|e| format!("保存导入私钥失败 {}: {}", key_path.display(), e))?;
        Ok(key_path)
    }

    pub fn get_projects(&self) -> Result<Vec<Project>, String> {
        let path = self.projects_path();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| format!("读取项目配置失败 {}: {}", path.display(), e))?;
        if content.trim().is_empty() {
            return Ok(Vec::new());
        }

        let mut projects: Vec<Project> = serde_json::from_str(&content)
            .map_err(|e| format!("解析项目配置失败 {}: {}", path.display(), e))?;
        for project in &mut projects {
            project.normalize();
            if project.auth_method == AUTH_METHOD_PASSWORD && !project.password_or_key.is_empty() {
                if is_local_encrypted_secret(&project.password_or_key) {
                    project.password_or_key = decrypt_local_secret(&project.password_or_key)?;
                } else {
                    return Err(format!(
                        "项目「{}」包含未加密的服务器密码，请重新创建或导入加密配置",
                        project.name
                    ));
                }
            }
        }
        Ok(projects)
    }

    pub fn save_project(&self, mut project: Project) -> Result<Project, String> {
        project.normalize();
        let mut projects = self.get_projects()?;

        if let Some(existing) = projects.iter_mut().find(|item| item.id == project.id) {
            *existing = project.clone();
        } else {
            projects.push(project.clone());
        }

        self.save_projects(&projects)?;
        Ok(project)
    }

    pub fn delete_project(&self, id: &str) -> Result<(), String> {
        let mut projects = self.get_projects()?;
        projects.retain(|project| project.id != id);
        self.save_projects(&projects)
    }

    pub fn save_projects(&self, projects: &[Project]) -> Result<(), String> {
        let projects = prepare_projects_for_save(projects)?;
        let content = serde_json::to_string_pretty(&projects)
            .map_err(|e| format!("序列化项目配置失败: {}", e))?;
        fs::write(self.projects_path(), content).map_err(|e| format!("保存项目配置失败: {}", e))
    }

    pub fn get_logs(&self) -> Result<Vec<LogEntry>, String> {
        let path = self.logs_path();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| format!("读取操作日志失败 {}: {}", path.display(), e))?;
        if content.trim().is_empty() {
            return Ok(Vec::new());
        }

        serde_json::from_str(&content)
            .map_err(|e| format!("解析操作日志失败 {}: {}", path.display(), e))
    }

    pub fn save_logs(&self, logs: &[LogEntry]) -> Result<(), String> {
        let content =
            serde_json::to_string_pretty(logs).map_err(|e| format!("序列化操作日志失败: {}", e))?;
        fs::write(self.logs_path(), content).map_err(|e| format!("保存操作日志失败: {}", e))
    }
}

fn prepare_projects_for_save(projects: &[Project]) -> Result<Vec<Project>, String> {
    projects
        .iter()
        .cloned()
        .map(|mut project| {
            project.normalize();
            if project.auth_method == AUTH_METHOD_PASSWORD
                && !project.password_or_key.is_empty()
                && !is_local_encrypted_secret(&project.password_or_key)
            {
                project.password_or_key = encrypt_local_secret(&project.password_or_key)?;
            }
            Ok(project)
        })
        .collect()
}

fn is_local_encrypted_secret(value: &str) -> bool {
    value.starts_with(LOCAL_SECRET_PREFIX_WITH_SEPARATOR)
}

fn encrypt_local_secret(secret: &str) -> Result<String, String> {
    let protected = protect_local_secret(secret.as_bytes())?;
    Ok(format!(
        "{}:{}",
        LOCAL_SECRET_PREFIX,
        hex_encode(&protected)
    ))
}

fn decrypt_local_secret(value: &str) -> Result<String, String> {
    let encrypted = value
        .strip_prefix(LOCAL_SECRET_PREFIX_WITH_SEPARATOR)
        .ok_or_else(|| "本地密码不是受支持的加密格式".to_string())?;
    let encrypted = hex_decode(encrypted).map_err(|e| format!("解析本地密码失败: {}", e))?;
    let decrypted = unprotect_local_secret(&encrypted)?;
    String::from_utf8(decrypted).map_err(|_| "本地密码解密后不是有效文本".to_string())
}

#[cfg(target_os = "windows")]
fn protect_local_secret(secret: &[u8]) -> Result<Vec<u8>, String> {
    windows_protect_secret(secret)
}

#[cfg(target_os = "windows")]
fn unprotect_local_secret(secret: &[u8]) -> Result<Vec<u8>, String> {
    windows_unprotect_secret(secret)
}

#[cfg(not(target_os = "windows"))]
fn protect_local_secret(secret: &[u8]) -> Result<Vec<u8>, String> {
    Ok(secret.iter().map(|byte| byte ^ 0xa5).collect())
}

#[cfg(not(target_os = "windows"))]
fn unprotect_local_secret(secret: &[u8]) -> Result<Vec<u8>, String> {
    Ok(secret.iter().map(|byte| byte ^ 0xa5).collect())
}

fn hex_encode(bytes: &[u8]) -> String {
    const CHARS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(CHARS[(byte >> 4) as usize] as char);
        encoded.push(CHARS[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn hex_decode(value: &str) -> Result<Vec<u8>, String> {
    if value.len() % 2 != 0 {
        return Err("长度不是偶数".to_string());
    }

    let mut decoded = Vec::with_capacity(value.len() / 2);
    let bytes = value.as_bytes();
    for chunk in bytes.chunks_exact(2) {
        let high = hex_value(chunk[0])?;
        let low = hex_value(chunk[1])?;
        decoded.push((high << 4) | low);
    }
    Ok(decoded)
}

fn hex_value(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(format!("非法十六进制字符: {}", byte as char)),
    }
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct DataBlob {
    cb_data: u32,
    pb_data: *mut u8,
}

#[cfg(target_os = "windows")]
#[link(name = "crypt32")]
unsafe extern "system" {
    fn CryptProtectData(
        p_data_in: *mut DataBlob,
        sz_data_descr: *const u16,
        p_optional_entropy: *mut DataBlob,
        pv_reserved: *mut std::ffi::c_void,
        p_prompt_struct: *mut std::ffi::c_void,
        dw_flags: u32,
        p_data_out: *mut DataBlob,
    ) -> i32;

    fn CryptUnprotectData(
        p_data_in: *mut DataBlob,
        ppsz_data_descr: *mut *mut u16,
        p_optional_entropy: *mut DataBlob,
        pv_reserved: *mut std::ffi::c_void,
        p_prompt_struct: *mut std::ffi::c_void,
        dw_flags: u32,
        p_data_out: *mut DataBlob,
    ) -> i32;
}

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn LocalFree(h_mem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
}

#[cfg(target_os = "windows")]
fn windows_protect_secret(secret: &[u8]) -> Result<Vec<u8>, String> {
    windows_crypt_secret(secret, true)
}

#[cfg(target_os = "windows")]
fn windows_unprotect_secret(secret: &[u8]) -> Result<Vec<u8>, String> {
    windows_crypt_secret(secret, false)
}

#[cfg(target_os = "windows")]
fn windows_crypt_secret(secret: &[u8], protect: bool) -> Result<Vec<u8>, String> {
    const CRYPTPROTECT_UI_FORBIDDEN: u32 = 0x1;

    let mut input = DataBlob {
        cb_data: secret
            .len()
            .try_into()
            .map_err(|_| "本地密码数据过大，无法处理".to_string())?,
        pb_data: secret.as_ptr() as *mut u8,
    };
    let mut output = DataBlob {
        cb_data: 0,
        pb_data: std::ptr::null_mut(),
    };

    let ok = unsafe {
        if protect {
            CryptProtectData(
                &mut input,
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &mut input,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        }
    };

    if ok == 0 {
        return Err(if protect {
            "加密本地服务器密码失败".to_string()
        } else {
            "解密本地服务器密码失败，可能不是当前 Windows 用户创建的配置".to_string()
        });
    }

    let bytes =
        unsafe { std::slice::from_raw_parts(output.pb_data, output.cb_data as usize).to_vec() };
    unsafe {
        LocalFree(output.pb_data as *mut std::ffi::c_void);
    }
    Ok(bytes)
}

fn safe_path_component(value: &str, fallback: &str) -> String {
    let safe = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();

    let trimmed = safe.trim_matches('.');
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn app_config_dir() -> Result<PathBuf, String> {
    if cfg!(target_os = "windows") {
        if let Some(roaming) = std::env::var_os("APPDATA") {
            return Ok(PathBuf::from(roaming).join("com.deploydesk.app"));
        }
    }

    dirs::config_dir()
        .map(|path| path.join("com.deploydesk.app"))
        .ok_or_else(|| "无法定位系统配置目录".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("deploydesk-storage-test-{}", Uuid::new_v4()));
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
    fn local_secret_protection_round_trips_without_plaintext() {
        let password = "server-password-123";
        let encrypted = encrypt_local_secret(password).unwrap();

        assert!(encrypted.starts_with(LOCAL_SECRET_PREFIX_WITH_SEPARATOR));
        assert!(!encrypted.contains(password));
        assert_eq!(decrypt_local_secret(&encrypted).unwrap(), password);
    }

    #[test]
    fn save_projects_encrypts_password_auth_in_projects_json() {
        let dir = TestDir::new();
        let storage = Storage {
            app_dir: dir.path.clone(),
        };
        let mut project = Project::blank();
        project.name = "demo".to_string();
        project.auth_method = AUTH_METHOD_PASSWORD.to_string();
        project.password_or_key = "plain-server-password".to_string();

        storage.save_projects(&[project]).unwrap();

        let content = fs::read_to_string(storage.projects_path()).unwrap();
        assert!(!content.contains("plain-server-password"));
        assert!(content.contains(LOCAL_SECRET_PREFIX));

        let projects = storage.get_projects().unwrap();
        assert_eq!(projects[0].password_or_key, "plain-server-password");
    }

    #[test]
    fn get_projects_rejects_legacy_plaintext_password() {
        let dir = TestDir::new();
        let storage = Storage {
            app_dir: dir.path.clone(),
        };
        let mut project = Project::blank();
        project.name = "legacy".to_string();
        project.auth_method = AUTH_METHOD_PASSWORD.to_string();
        project.password_or_key = "legacy-plain-password".to_string();
        fs::write(
            storage.projects_path(),
            serde_json::to_string_pretty(&vec![project]).unwrap(),
        )
        .unwrap();

        let error = storage.get_projects().unwrap_err();
        assert!(error.contains("未加密的服务器密码"));
    }
}

use tauri::Manager;
use zhangmenriji::config::AppConfig;

/// 获取配置文件路径（系统用户设置目录下的 config.json）
fn config_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("无法获取配置目录: {e}"))?;
    Ok(dir.join("config.json"))
}

/// 获取当前配置（从文件读取，不存在则返回默认值）
#[tauri::command]
fn get_config(app: tauri::AppHandle) -> Result<AppConfig, String> {
    let path = config_path(&app)?;
    if path.exists() {
        AppConfig::load_from_file(&path).ok_or_else(|| "配置文件损坏".into())
    } else {
        Ok(AppConfig::default())
    }
}

/// 保存配置到文件（需重启应用生效）
#[tauri::command]
fn save_config(app: tauri::AppHandle, config: AppConfig) -> Result<(), String> {
    let path = config_path(&app)?;
    config.save_to_file(&path)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // —— 从系统配置目录加载配置，注入环境变量 ——
            let path = config_path(&app.handle())?;

            // 确保配置目录存在
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let app_config = if path.exists() {
                AppConfig::load_from_file(&path).unwrap_or_default()
            } else {
                // 首次运行：生成默认配置文件
                let default_cfg = AppConfig::default();
                let _ = default_cfg.save_to_file(&path);
                default_cfg
            };

            // 注入环境变量（后端通过 env::var 读取）
            app_config.apply_to_env();

            // —— 启动后端服务 ——
            std::thread::spawn(move || {
                let runtime = tokio::runtime::Runtime::new()
                    .expect("Tokio 运行时创建失败");
                runtime.block_on(async {
                    zhangmenriji::run_server(Some(&path))
                        .await
                        .expect("后端启动失败");
                });
            });

            // 给数据库连接、迁移和 HTTP 监听留出启动时间
            std::thread::sleep(std::time::Duration::from_millis(800));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_config, save_config])
        .run(tauri::generate_context!())
        .expect("Tauri 启动失败");
}

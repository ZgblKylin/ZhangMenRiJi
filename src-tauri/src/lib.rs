#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 桌面端的 API 固定为本机 3000 端口，与前端 API_BASE 保持一致。
    std::env::set_var("SERVER_HOST", "127.0.0.1");
    std::env::set_var("SERVER_PORT", "3000");

    std::thread::spawn(|| {
        let runtime = tokio::runtime::Runtime::new().expect("Tokio 运行时创建失败");
        runtime.block_on(async {
            zhangmenriji::run_server().await.expect("后端启动失败");
        });
    });

    // 给数据库连接、迁移和 HTTP 监听留出启动时间。
    std::thread::sleep(std::time::Duration::from_millis(800));

    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("Tauri 启动失败");
}

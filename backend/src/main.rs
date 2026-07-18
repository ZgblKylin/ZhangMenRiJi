#[tokio::main]
async fn main() {
    let config_path = zhangmenriji::config::default_config_path();
    zhangmenriji::run_server(Some(&config_path))
        .await
        .expect("服务器启动失败");
}

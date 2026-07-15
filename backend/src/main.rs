#[tokio::main]
async fn main() {
    zhangmenriji::run_server().await.expect("服务器启动失败");
}

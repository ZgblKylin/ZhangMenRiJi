#[tokio::main]
async fn main() {
    zhangmenriji::run_server(None)
        .await
        .expect("服务器启动失败");
}

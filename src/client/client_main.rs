use syncplay::client::{create_client};
use simple_logger::SimpleLogger;
use log::*;
use syncplay::server::{Res, OK};
use tokio::time::Duration;

#[tokio::main]
pub async fn main() -> Res {
    SimpleLogger::new().with_level(LevelFilter::Info).init().unwrap();
    // Bind a server socket
    let client = create_client("127.0.0.1:5135").await?;
    client.create_session("HHHHH".to_string()).await;
    client.start_session().await;
    client.play().await;

    tokio::time::sleep(Duration::from_millis(1000)).await;
    OK
}

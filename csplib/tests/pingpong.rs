use csplib::*;

#[csplib::process]
struct Ping {
    #[input]
    x: String,
    #[output]
    y: String,
}
async fn run_ping(inner: PingInner) -> Result<String> {
    let s = "ping".to_owned();
    inner.y.put(s)?;
    tokio::task::yield_now().await;
    let y = inner.x.reader().get().await?;
    Ok(y)
}
#[csplib::process]
struct Pong {
    #[input]
    x: String,
    #[output]
    y: String,
}
async fn run_pong(inner: PongInner) -> Result<()> {
    let x = inner.x.reader().get().await?;
    tokio::task::yield_now().await;
    let s = format!("{}-pong", x);
    inner.y.put(s).unwrap();
    Ok(())
}

#[tokio::test]
async fn pingpong() {
    let (ping, ping_inner) = Ping::new();
    let (pong, pong_inner) = Pong::new();

    tokio::spawn(connect(ping.y.reader(), pong.x));
    tokio::spawn(connect(pong.y.reader(), ping.x));

    tokio::spawn(run_pong(pong_inner));
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let y = tokio::spawn(run_ping(ping_inner)).await.unwrap().unwrap();
    assert_eq!(y, "ping-pong")
}

use csplib::*;

#[csplib::process]
struct P1 {
    #[input]
    a: i32,
    #[output]
    b: i32,
}
// λx. x+2
async fn run_p1(inner: P1Inner) -> Result<()> {
    let x = inner.a.reader().get().await?;
    inner.b.put(x + 2)?;
    Ok(())
}
#[csplib::process]
struct P2 {
    #[input]
    a: i32,
    #[output]
    b: i32,
}
// λx. x*2
async fn run_p2(inner: P2Inner) -> Result<()> {
    let x = inner.a.reader().get().await?;
    // Emulating expensive I/O
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    inner.b.put(x * 2)?;
    Ok(())
}
#[csplib::process]
struct P3 {
    #[input]
    a: i32,
    #[input]
    b: i32,
    #[output]
    c: i32,
}
// λxy. x*y
async fn run_p3(inner: P3Inner) -> Result<()> {
    let x = inner.a.reader().get().await?;
    let y = inner.b.reader().get().await?;
    inner.c.put(x * y)?;
    Ok(())
}

#[tokio::test]
async fn computational_graph() {
    let (main_w, main_r) = channel();
    let (p1, p1_inner) = P1::new();
    let (p2, p2_inner) = P2::new();
    let (p3, p3_inner) = P3::new();

    tokio::spawn(run_p1(p1_inner));
    tokio::spawn(run_p2(p2_inner));
    tokio::spawn(run_p3(p3_inner));

    tokio::spawn(connect(main_r.reader(), p1.a));
    tokio::spawn(connect(main_r.reader(), p2.a));
    tokio::spawn(connect(p1.b.reader(), p3.a));
    tokio::spawn(connect(p2.b.reader(), p3.b));

    // Wait for all spawnings.
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    main_w.put(1).unwrap();
    let ans = p3.c.reader().get().await.unwrap();
    assert_eq!(ans, 6);
}

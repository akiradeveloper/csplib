use csplib::*;

#[csplib::process]
struct And {
    #[input]
    a: bool,
    #[input]
    b: bool,
    #[output]
    c: bool,
}
async fn run_and(inner: AndInner) -> Result<()> {
    let a = inner.a_r.reader();
    let b = inner.b_r.reader();
    let (a, b) = tokio::try_join!(a.get(), b.get())?;
    let c = a & b;
    inner.c_w.put(c)?;
    Ok(())
}

#[tokio::test]
async fn circuit() {
    let (and1, and_inner1) = And::new();
    tokio::spawn(run_and(and_inner1));
    let (and2, and_inner2) = And::new();
    tokio::spawn(run_and(and_inner2));

    tokio::spawn(connect(and1.c_r.reader(), and2.a_w));

    // Wait for all spawnings.
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    and1.a_w.put(true).unwrap();
    and1.b_w.put(true).unwrap();
    let and1c = and1.c_r.reader().get().await.unwrap();
    assert_eq!(and1c, true);

    and2.b_w.put(false).unwrap();
    let and2c = and2.c_r.reader().get().await.unwrap();
    assert_eq!(and2c, false);
}

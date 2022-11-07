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
impl AndInner {
    async fn run(self) -> Result<()> {
        let a = self.a_r.reader();
        let b = self.b_r.reader();
        let (a, b) = tokio::try_join!(a.get(), b.get())?;
        let c = a & b;
        self.c_w.put(c)?;
        Ok(())
    }
}

#[tokio::test]
async fn circuit() {
    let (and1, and_inner1) = And::new();
    tokio::spawn(and_inner1.run());
    let (and2, and_inner2) = And::new();
    tokio::spawn(and_inner2.run());
    tokio::spawn({
        let r = and1.c_r.reader();
        connect(r, and2.a_w)
    });

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

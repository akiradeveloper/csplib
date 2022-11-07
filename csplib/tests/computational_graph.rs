use csplib::*;

#[tokio::test]
async fn computational_graph() {
    let (w1, ch1) = channel();
    let (w2, ch2) = channel();
    let (w3, ch3) = channel();
    let (w4, ch4) = channel();
    // λx. x+2
    tokio::spawn({
        let r1 = ch1.reader();
        async move {
            let x = r1.get().await.unwrap();
            w2.put(x + 2).unwrap();
        }
    });
    // λx. x*2
    tokio::spawn({
        let r1 = ch1.reader();
        async move {
            // Emulating expensive I/O
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            let x = r1.get().await.unwrap();
            w3.put(x * 2).unwrap();
        }
    });
    // λxy. x*y
    tokio::spawn({
        let r2 = ch2.reader();
        let r3 = ch3.reader();
        async move {
            let (x, y) = tokio::try_join!(r2.get(), r3.get()).unwrap();
            w4.put(x * y).unwrap();
        }
    });
    w1.put(1).unwrap();
    let r4 = ch4.reader();
    let ans = r4.get().await.unwrap();
    assert_eq!(ans, 6);
}

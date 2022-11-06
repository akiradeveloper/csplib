use csplib::*;

#[tokio::test]
async fn pingpong() {
	let (w1, ch1) = channel();
	let (w2, ch2) = channel();
	tokio::spawn({
		let r1 = ch1.reader();
		async move {
			let x = r1.get().await.unwrap();
			tokio::task::yield_now().await;
			let s = format!("{}pong", x);
			w2.put(s).unwrap();
		}
	});
	let y = tokio::spawn({
		let r2 = ch2.reader();
		async move {
			let x = "ping".to_owned();
			w1.put(x).unwrap();
			tokio::task::yield_now().await;
			let y = r2.get().await.unwrap();
			y
		}
	})
	.await
	.unwrap();
	assert_eq!(y, "pingpong")
}
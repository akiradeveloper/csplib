use csplib::*;

struct And {
	pub a: Writer<bool>,
	pub b: Writer<bool>,
	pub c: Channel<bool>,
}
struct AndInner {
	a_ch: Channel<bool>,
	b_ch: Channel<bool>,
	c_w: Writer<bool>,
}
impl And {
	pub fn new() -> (And, AndInner) {
		let (a, a_ch) = channel();
		let (b, b_ch) = channel();
		let (c, c_ch) = channel();
		let out = And {
			a: a,
			b: b,
			c: c_ch,
		};
		let runner = AndInner { a_ch, b_ch, c_w: c };
		(out, runner)
	}
}
impl AndInner {
	async fn run(self) -> Result<()> {
		let a = self.a_ch.reader();
		let b = self.b_ch.reader();
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
		let r = and1.c.reader();
		connect(r, and2.a)
	});

	// Wait for all spawnings.
	tokio::time::sleep(std::time::Duration::from_secs(1)).await;

	and1.a.put(true).unwrap();
	and1.b.put(true).unwrap();
	let and1c = and1.c.reader().get().await.unwrap();
	assert_eq!(and1c, true);

	and2.b.put(false).unwrap();
	let and2c = and2.c.reader().get().await.unwrap();
	assert_eq!(and2c, false);
}
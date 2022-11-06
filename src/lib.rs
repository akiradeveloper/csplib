use crossbeam::queue::SegQueue;
use futures::channel::oneshot;
use std::sync::Arc;
use thiserror::Error;

type SenderQueue<T> = Arc<SegQueue<oneshot::Sender<T>>>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("receiver is failed.")]
    ReceiverFailed,
    #[error("sender is failed.")]
    SenderFailed,
}
type Result<T> = std::result::Result<T, Error>;

pub struct Writer<T> {
    q: SenderQueue<T>,
}
impl<T: Clone> Writer<T> {
    pub fn put(self, a: T) -> Result<()> {
        let q = self.q;
        while let Some(sender) = q.pop() {
            sender.send(a.clone()).map_err(|_| Error::ReceiverFailed)?;
        }
        Ok(())
    }
}
pub struct Reader<T> {
    inner: oneshot::Receiver<T>,
}
impl<T> Reader<T> {
    pub async fn get(self) -> Result<T> {
        let o = self.inner.await.map_err(|_| Error::SenderFailed)?;
        Ok(o)
    }
}
pub struct Channel<T> {
    q: SenderQueue<T>,
}
impl<T> Channel<T> {
    fn new() -> Self {
        Self {
            q: Arc::new(SegQueue::new()),
        }
    }
    fn writer(&self) -> Writer<T> {
        Writer { q: self.q.clone() }
    }
    pub fn reader(&self) -> Reader<T> {
        let (sender, receiver) = oneshot::channel();
        self.q.push(sender);
        Reader { inner: receiver }
    }
}

pub async fn connect<T: Clone>(reader: Reader<T>, writer: Writer<T>) -> Result<()> {
    let x = reader.get().await?;
    writer.put(x)?;
    Ok(())
}

pub fn channel<T>() -> (Writer<T>, Channel<T>) {
    let ch = Channel::new();
    let w = ch.writer();
    (w, ch)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_pingpong() {
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
    #[tokio::test]
    async fn test_computational_graph() {
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
    #[tokio::test]
    async fn test_circuit() {
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
        let (and1, and_run1) = And::new();
        tokio::spawn(and_run1.run());
        let (and2, and_run2) = And::new();
        tokio::spawn(and_run2.run());
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
}

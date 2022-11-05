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

pub fn channel<T>() -> (Channel<T>, Writer<T>) {
    let ch = Channel::new();
    let w = ch.writer();
    (ch, w)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_pingpong() {
        let (ch1, w1) = channel();
        let (ch2, w2) = channel();
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
        let (ch1, w1) = channel();
        let (ch2, w2) = channel();
        let (ch3, w3) = channel();
        let (ch4, w4) = channel();
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
}

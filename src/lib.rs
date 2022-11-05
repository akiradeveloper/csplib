use crossbeam::queue::SegQueue;
use futures::channel::oneshot;
use std::sync::Arc;
use thiserror::Error;

type SenderQueue<T> = Arc<SegQueue<oneshot::Sender<T>>>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("receiver is failed.")]
    ReceiverFailure,
    #[error("sender is failed.")]
    SenderFailure,
}
type Result<T> = std::result::Result<T, Error>;

pub struct In<T> {
    q: SenderQueue<T>,
}
impl<T: Clone> In<T> {
    pub fn put(self, a: T) -> Result<()> {
        let q = self.q;
        while let Some(sender) = q.pop() {
            sender.send(a.clone()).map_err(|e| Error::ReceiverFailure)?;
        }
        Ok(())
    }
}
pub struct Out<T> {
    inner: oneshot::Receiver<T>,
}
impl<T> Out<T> {
    pub async fn get(self) -> Result<T> {
        let o = self.inner.await.map_err(|e| Error::SenderFailure)?;
        Ok(o)
    }
}
pub struct Channel<T> {
    q: SenderQueue<T>,
}
impl<T> Channel<T> {
    pub fn new() -> Self {
        Self {
            q: Arc::new(SegQueue::new()),
        }
    }
    pub fn input(&self) -> In<T> {
        In { q: self.q.clone() }
    }
    pub fn output(&self) -> Out<T> {
        let (sender, receiver) = oneshot::channel();
        self.q.push(sender);
        Out { inner: receiver }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_pingpong() {
        let ch1 = Channel::new();
        let ch2 = Channel::new();
        tokio::spawn({
            let out1 = ch1.output();
            let in2 = ch2.input();
            async move {
                let x = out1.get().await.unwrap();
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                let s = format!("{}pong", x);
                in2.put(s).unwrap();
            }
        });
        let y = tokio::spawn({
            let in1 = ch1.input();
            let out2 = ch2.output();
            async move {
                let x = "ping".to_owned();
                in1.put(x).unwrap();
                let y = out2.get().await.unwrap();
                y
            }
        })
        .await
        .unwrap();
        assert_eq!(y, "pingpong")
    }
    #[tokio::test]
    async fn test_computational_graph() {
        let ch1 = Channel::new();
        let ch2 = Channel::new();
        let ch3 = Channel::new();
        let ch4 = Channel::new();
        // λx. x+2
        tokio::spawn({
            let out1 = ch1.output();
            let in2 = ch2.input();
            async move {
                let x = out1.get().await.unwrap();
                in2.put(x + 2).unwrap();
            }
        });
        // λx. x*2
        tokio::spawn({
            let out1 = ch1.output();
            let in3 = ch3.input();
            async move {
                // Emulating expensive I/O
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                let x = out1.get().await.unwrap();
                in3.put(x * 2).unwrap();
            }
        });
        // λxy. x*y
        tokio::spawn({
            let out2 = ch2.output();
            let out3 = ch3.output();
            let in4 = ch4.input();
            async move {
                let (x, y) = tokio::try_join!(out2.get(), out3.get()).unwrap();
                in4.put(x * y).unwrap();
            }
        });
        let in1 = ch1.input();
        in1.put(1).unwrap();
        let out4 = ch4.output();
        let ans = out4.get().await.unwrap();
        assert_eq!(ans, 6);
    }
}

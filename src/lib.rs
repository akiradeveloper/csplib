use crossbeam::queue::SegQueue;
use futures::channel::oneshot;
use std::sync::Arc;
use thiserror::Error;

type SenderQueue<T> = Arc<SegQueue<oneshot::Sender<T>>>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("no receiver is ready.")]
    ReceiverNotReady,
    #[error("receiver is failed.")]
    ReceiverFailed,
    #[error("sender is failed.")]
    SenderFailed,
}
pub type Result<T> = std::result::Result<T, Error>;

pub struct Writer<T> {
    q: SenderQueue<T>,
}
impl<T: Clone> Writer<T> {
    pub fn put(self, a: T) -> Result<()> {
        let q = self.q;
        if q.is_empty() {
            return Err(Error::ReceiverNotReady);
        }
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

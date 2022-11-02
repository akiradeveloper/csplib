use crossbeam::queue::SegQueue;
use futures::channel::oneshot;
use std::sync::Arc;

type SenderQueue<T> = Arc<SegQueue<oneshot::Sender<T>>>;

pub struct In<T> {
    q: SenderQueue<T>,
}
impl<T: Clone> In<T> {
    pub fn put(self, a: T) {
        let q = self.q;
        while let Some(sender) = q.pop() {
            sender.send(a.clone());
        }
    }
}
pub type Out<T> = oneshot::Receiver<T>;
pub struct Node<T> {
    q: SenderQueue<T>,
}
impl<T> Node<T> {
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
        receiver
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test() {
        let n1 = Node::new();
        let n2 = Node::new();
        let n3 = Node::new();
        let n4 = Node::new();
        // λx. x+2
        tokio::spawn({
            let out1 = n1.output();
            let in2 = n2.input();
            async move {
                let x = out1.await.unwrap();
                in2.put(x + 2);
            }
        });
        // λx. x*2
        tokio::spawn({
            let out1 = n1.output();
            let in3 = n3.input();
            async move {
                let x = out1.await.unwrap();
                in3.put(x * 2);
            }
        });
        // λxy. x*y
        tokio::spawn({
            let out2 = n2.output();
            let out3 = n3.output();
            let in4 = n4.input();
            async move {
                let (x, y) = tokio::try_join!(out2, out3).unwrap();
                in4.put(x * y);
            }
        });
        let in1 = n1.input();
        in1.put(1);
        let out4 = n4.output();
        let ans = out4.await.unwrap();
        assert_eq!(ans, 6);
    }
}

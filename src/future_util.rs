use std::future::Future;
use tokio::sync::{mpsc, oneshot};

pub async fn to_channel<F: 'static + Future<Output = T> + Send, T: 'static + Send>(
    f: F,
) -> oneshot::Receiver<T> {
    let (ch_s, ch_r) = oneshot::channel();
    tokio::spawn(_to_channel(f, ch_s));
    ch_r
}

async fn _to_channel<F: Future<Output = T>, T: Send>(f: F, ch: oneshot::Sender<T>) {
    let r = f.await;
    ch.send(r).ok();
}

pub async fn on_sender<F: 'static + Future<Output = T> + Send, T: 'static + Send>(
    f: F,
    ch: mpsc::Sender<T>,
) {
    tokio::spawn(_on_sender(f, ch));
}

async fn _on_sender<F: Future<Output = T>, T: Send>(f: F, ch: mpsc::Sender<T>) {
    ch.send(f.await).await.ok();
}

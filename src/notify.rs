use actix_web::http::StatusCode;
use actix_web::{Error, ResponseError};
use derive_more::Display;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::timeout;

#[derive(Debug, Display, Clone)]
#[display(fmt = "internal notification error")]
struct NotifierError;

impl ResponseError for NotifierError {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

pub struct Subscription {
    receiver: broadcast::Receiver<usize>,
    clock: usize,
}

pub struct Notifier {
    sender: broadcast::Sender<usize>,
    clock: AtomicUsize,
}

impl Notifier {
    pub fn new() -> Self {
        Notifier {
            sender: broadcast::channel(8).0,
            clock: AtomicUsize::new(1),
        }
    }

    pub fn send(&self) {
        let _ = self.sender.send(self.clock.fetch_add(1, Ordering::SeqCst));
    }

    pub fn subscribe(&self) -> Subscription {
        Subscription {
            receiver: self.sender.subscribe(),
            clock: self.clock.load(Ordering::SeqCst),
        }
    }
}

const TIMEOUT_DURATION: Duration = Duration::from_secs(5);

impl Subscription {
    pub async fn wait(&mut self, since: usize) -> actix_web::Result<usize> {
        if self.clock > since {
            Ok(self.clock)
        } else {
            match timeout(TIMEOUT_DURATION, self.receiver.recv()).await {
                Ok(Ok(clk)) => Ok(clk),
                Ok(Err(_)) => Err(Error::from(NotifierError {})),
                Err(_) => Ok(self.clock),
            }
        }
    }
}

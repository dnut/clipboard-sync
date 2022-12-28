use futures::Future;
use std::ptr;
use std::task::Poll;
use std::{
    sync::{Arc, Mutex},
    thread,
};

pub struct SideThreadFuture<T>(Arc<Mutex<Option<Sendable<T>>>>);
struct Sendable<T>(T);
unsafe impl<T> Send for Sendable<T> {}

impl<T> Future for SideThreadFuture<T> {
    type Output = T;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        unsafe {
            match self.0.lock().unwrap().as_mut() {
                Some(Sendable(x)) => Poll::Ready(ptr::read(x)),
                None => Poll::Pending,
            }
        }
    }
}

pub fn asyncify<T: 'static, F: Fn() -> T + Send + 'static>(job: F) -> SideThreadFuture<T> {
    let result = Arc::new(Mutex::new(None));
    let movable = result.clone();
    thread::spawn(move || {
        let ret = job();
        let mut mx = movable.lock().unwrap();
        *mx = Some(Sendable(ret));
    });

    SideThreadFuture(result)
}

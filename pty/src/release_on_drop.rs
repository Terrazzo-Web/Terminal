use futures::channel::oneshot;
use tracing::warn;

pub struct ReleaseOnDrop<T> {
    value: Option<T>,
    on_drop: Option<oneshot::Sender<T>>,
}

impl<T> ReleaseOnDrop<T> {
    pub fn new(value: T) -> (Self, oneshot::Receiver<T>) {
        let (tx, rx) = oneshot::channel();
        (
            Self {
                value: Some(value),
                on_drop: Some(tx),
            },
            rx,
        )
    }
}

impl<T> AsMut<T> for ReleaseOnDrop<T> {
    fn as_mut(&mut self) -> &mut T {
        let value = self.value.as_mut();
        unsafe { value.unwrap_unchecked() }
    }
}

impl<T> AsRef<T> for ReleaseOnDrop<T> {
    fn as_ref(&self) -> &T {
        let value = self.value.as_ref();
        unsafe { value.unwrap_unchecked() }
    }
}

impl<T> Drop for ReleaseOnDrop<T> {
    fn drop(&mut self) {
        let value = self.value.take().expect("ReleaseOnDrop: double drop?");
        if let Some(on_drop) = self.on_drop.take() {
            let result = on_drop.send(value);
            if cfg!(debug_assertions) && result.is_err() {
                warn!("ReleaseOnDrop: Unable to release on drop");
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use std::time::Duration;

    use tokio::time::timeout;

    use super::ReleaseOnDrop;

    #[tokio::test]
    async fn release_on_drop() -> Result<(), Box<dyn std::error::Error>> {
        #[derive(Debug, PartialEq, Eq)]
        struct Thing(&'static str);
        let (thing, rx) = ReleaseOnDrop::new(Thing("hello world!"));
        drop(thing);
        let thing = rx.await?;
        assert_eq!(Thing("hello world!"), thing);
        Ok(())
    }

    #[tokio::test]
    async fn no_drop_timeout() -> Result<(), Box<dyn std::error::Error>> {
        #[derive(Debug, PartialEq, Eq)]
        struct Thing(&'static str);
        let (_thing, rx) = ReleaseOnDrop::new(Thing("hello world!"));
        timeout(Duration::from_millis(100), rx)
            .await
            .expect_err("Should timeout");
        Ok(())
    }

    #[tokio::test]
    async fn drop_rx() -> Result<(), Box<dyn std::error::Error>> {
        #[derive(Debug, PartialEq, Eq)]
        struct Thing(&'static str);
        let (thing, rx) = ReleaseOnDrop::new(Thing("hello world!"));
        drop(rx);
        drop(thing);
        Ok(())
    }
}

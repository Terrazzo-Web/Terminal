use std::pin::Pin;

use futures::channel::oneshot;
use pin_project::pin_project;
use pin_project::pinned_drop;
use tracing::warn;

#[pin_project(PinnedDrop, project = ReleaseOnDropProj)]
pub struct ReleaseOnDrop<T> {
    #[pin]
    value: OptionPinned<T>,
    on_drop: Option<oneshot::Sender<T>>,
}

#[pin_project(project = OptionPinnedProj)]
#[derive(Default)]
enum OptionPinned<T> {
    Some(#[pin] T),
    #[default]
    None,
}

impl<T> Default for OptionPinnedProj<'_, T> {
    fn default() -> Self {
        Self::None
    }
}

impl<T> ReleaseOnDrop<T> {
    pub fn new(value: T) -> (Self, oneshot::Receiver<T>) {
        let (tx, rx) = oneshot::channel();
        (
            Self {
                value: OptionPinned::Some(value),
                on_drop: Some(tx),
            },
            rx,
        )
    }
}

impl<T> AsRef<T> for ReleaseOnDrop<T> {
    fn as_ref(&self) -> &T {
        let OptionPinned::Some(value) = &self.value else {
            unreachable!()
        };
        return value;
    }
}

impl<T> AsMut<T> for ReleaseOnDrop<T> {
    fn as_mut(&mut self) -> &mut T {
        let OptionPinned::Some(value) = &mut self.value else {
            unreachable!()
        };
        return value;
    }
}

impl<T> ReleaseOnDropProj<'_, T> {
    pub fn as_pin(&mut self) -> Pin<&mut T> {
        let OptionPinnedProj::Some(value) = self.value.as_mut().project() else {
            unreachable!()
        };
        return value;
    }
}

#[pinned_drop]
impl<T> PinnedDrop for ReleaseOnDrop<T> {
    fn drop(self: Pin<&mut Self>) {
        let this = self.project();
        let value = std::mem::take(&mut this.value.project());
        let OptionPinnedProj::Some(value) = value else {
            panic!("ReleaseOnDrop: double drop?");
        };
        if let Some(on_drop) = this.on_drop.take() {
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

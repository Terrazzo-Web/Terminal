#![cfg(feature = "server")]
#![cfg(feature = "text-editor")]

use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;

use tokio::sync::oneshot;
use tracing::Instrument as _;

pub struct Throttle<F> {
    callback: F,
    signal: Mutex<Option<oneshot::Sender<()>>>,
    pub min_delay_between_runs: Duration,
    pub min_delay_fraction: f64,
}

impl<F> Throttle<F> {
    pub fn new<I, O>(callback: F) -> Self
    where
        F: Fn(I) -> O,
        O: Future,
        O::Output: Default,
    {
        Self {
            callback,
            signal: None.into(),
            min_delay_between_runs: Duration::from_secs(1),
            min_delay_fraction: 0.5,
        }
    }

    pub async fn run<I, O>(self: &Arc<Self>, input: I) -> O::Output
    where
        F: Fn(I) -> O,
        F: Send + Sync + 'static,
        O: Future,
        O::Output: Default,
    {
        {
            let (tx, rx) = oneshot::channel();
            let signal = self.signal.lock().unwrap().replace(tx).is_some();
            if signal {
                if let Err(oneshot::error::RecvError { .. }) = rx.await {
                    // This run_again was stolen by a newer one.
                    return O::Output::default();
                }
            }
        };

        let callback = &self.callback;
        let start = Instant::now();
        let output = callback(input).await;
        let latency = start.elapsed();

        let this = self.clone();
        let cooldown_task = async move {
            let cooldown = Ord::max(
                Duration::from_secs_f64(latency.as_secs_f64() * this.min_delay_fraction),
                this.min_delay_between_runs,
            );
            let () = tokio::time::sleep(cooldown).await;
            let (tx, _rx) = oneshot::channel();
            let mut lock = this.signal.lock().unwrap();
            if let Some(signal) = lock.replace(tx) {
                if let Err(()) = signal.send(()) {
                    *lock = None;
                }
            }
        };
        tokio::spawn(cooldown_task.in_current_span());

        return output;
    }
}

#[cfg(test)]
mod tests {

    use std::sync::Arc;
    use std::time::Duration;
    use std::time::Instant;

    use trz_gateway_common::tracing::test_utils::enable_tracing_for_tests;

    use super::Throttle;

    #[tokio::test]
    async fn sequential() {
        enable_tracing_for_tests();
        let throttle = Arc::new(Throttle {
            min_delay_between_runs: Duration::from_millis(500),
            ..Throttle::new(async |x: i32| Some(x * x))
        });

        let start = Instant::now();
        assert_eq!(Some(4), throttle.run(2).await);
        assert!(start.elapsed() < throttle.min_delay_between_runs);

        let start = Instant::now();
        assert_eq!(Some(4), throttle.run(2).await);
        assert!(start.elapsed() >= throttle.min_delay_between_runs);

        let start = Instant::now();
        assert_eq!(Some(4), throttle.run(2).await);
        assert!(start.elapsed() >= throttle.min_delay_between_runs);
    }

    #[tokio::test]
    async fn parallel() {
        enable_tracing_for_tests();
        let throttle = Arc::new(Throttle {
            min_delay_between_runs: Duration::from_millis(500),
            ..Throttle::new(async |x: i32| Some(x * x))
        });

        let a = async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            throttle.run(1).await
        };
        let b = async {
            tokio::time::sleep(Duration::from_millis(200)).await;
            throttle.run(2).await
        };
        let c = async {
            tokio::time::sleep(Duration::from_millis(300)).await;
            throttle.run(3).await
        };
        let (a, b, c) = tokio::join!(a, b, c);
        assert_eq!((Some(1), None, Some(9)), (a, b, c));
    }
}

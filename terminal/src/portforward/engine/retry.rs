use std::ops::ControlFlow;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;
use std::time::Duration;
use std::time::Instant;

use futures::FutureExt;
use futures::Stream;
use futures::StreamExt as _;
use futures::channel::oneshot;
use futures::future::Shared;
use tokio::time::Timeout;
use tokio::time::error::Elapsed;
use tonic::Status;
use tracing::debug;
use tracing::warn;
use trz_gateway_common::retry_strategy::RetryStrategy;
use trz_gateway_server::server::Server;

use self::port_forward_service::bind::BindError;
use self::port_forward_service::bind::BindStream;
use self::protos::PortForwardAcceptResponse;
use crate::backend::client_service::port_forward_service;
use crate::backend::protos::terrazzo::portforward as protos;
use crate::portforward::schema::PortForward;
use crate::portforward::schema::PortForwardStatus;

pub struct BindStreamWithRetry(BindStreamWithRetryImpl);

impl Stream for BindStreamWithRetry {
    type Item = Result<PortForwardAcceptResponse, Status>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> StreamItem {
        self.0.poll_next_unpin(cx)
    }
}

impl BindStreamWithRetry {
    pub fn new(
        server: Arc<Server>,
        port_forward: PortForward,
        ask: Shared<oneshot::Receiver<()>>,
    ) -> Self {
        let info = Arc::new(StreamInfo {
            server,
            port_forward,
            ask,
        });
        Self(BindStreamWithRetryImpl::StreamingPrep(StreamingPrep {
            started_at: Instant::now(),
            retry_strategy: default_retry_strategy(),
            stream: Box::pin(info.get_bind_stream()),
            info,
        }))
    }
}

enum BindStreamWithRetryImpl {
    StreamingPrep(StreamingPrep),
    Streaming(Streaming),
    Retrying(Retrying),
}

type StreamingPrep =
    StreamingImpl<Pin<Box<dyn Future<Output = Result<BindStream, BindError>> + Send>>>;
type Streaming = StreamingImpl<BindStream>;

struct StreamingImpl<T> {
    started_at: Instant,
    retry_strategy: RetryStrategy,
    stream: T,
    info: Arc<StreamInfo>,
}

struct Retrying {
    sleep: Pin<Box<Timeout<Shared<oneshot::Receiver<()>>>>>,
    retry_strategy: RetryStrategy,
    info: Arc<StreamInfo>,
}

struct StreamInfo {
    server: Arc<Server>,
    port_forward: PortForward,
    ask: Shared<oneshot::Receiver<()>>,
}

type StreamItem = Poll<Option<Result<PortForwardAcceptResponse, Status>>>;

/// If the stream was up for more than [RETRY_COOLDOWN], the retry strategy resets to default.
static RETRY_COOLDOWN: Duration = Duration::from_secs(15);

/// The default retry strategy sleep, with exponential backoff
static DEFAULT_SLEEP: Duration = Duration::from_millis(100);

/// The max retry strategy sleep.
static MAX_SLEEP: Duration = Duration::from_secs(60);

fn default_retry_strategy() -> RetryStrategy {
    RetryStrategy::from(DEFAULT_SLEEP).exponential_backoff(2., MAX_SLEEP)
}

impl Stream for BindStreamWithRetryImpl {
    type Item = Result<PortForwardAcceptResponse, Status>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> StreamItem {
        loop {
            match self.poll_next_impl(cx) {
                ControlFlow::Continue(()) => (),
                ControlFlow::Break(result) => return result,
            }
        }
    }
}

impl BindStreamWithRetryImpl {
    fn poll_next_impl(&mut self, cx: &mut Context<'_>) -> ControlFlow<StreamItem> {
        *self = match self {
            BindStreamWithRetryImpl::StreamingPrep(streaming_prep) => {
                streaming_prep.poll_streaming_prep(cx)?
            }
            BindStreamWithRetryImpl::Streaming(streaming) => streaming.poll_streaming(cx)?,
            BindStreamWithRetryImpl::Retrying(retrying) => retrying.poll_next(cx)?,
        };
        ControlFlow::Continue(())
    }
}

impl StreamingPrep {
    fn poll_streaming_prep(
        &mut self,
        cx: &mut Context<'_>,
    ) -> ControlFlow<StreamItem, BindStreamWithRetryImpl> {
        match self.stream.poll_unpin(cx) {
            Poll::Ready(Ok(stream)) => {
                debug!("Got bind stream");
                self.info.port_forward.state.lock().status = PortForwardStatus::Up;
                ControlFlow::Continue(BindStreamWithRetryImpl::Streaming(Streaming {
                    started_at: self.started_at,
                    retry_strategy: self.retry_strategy.clone(),
                    stream,
                    info: self.info.clone(),
                }))
            }
            Poll::Ready(Err(error)) => {
                warn!("Failed to get bind stream, will retry: {error}");
                self.info.port_forward.state.lock().status =
                    PortForwardStatus::Failed(format!("{error} (will retry)"));
                let mut retry_strategy = self.retry_strategy.clone();
                let sleep = Box::pin(tokio::time::timeout(
                    retry_strategy.delay(),
                    self.info.ask.clone(),
                ));
                ControlFlow::Continue(BindStreamWithRetryImpl::Retrying(Retrying {
                    sleep,
                    retry_strategy,
                    info: self.info.clone(),
                }))
            }
            Poll::Pending => ControlFlow::Break(Poll::Pending),
        }
    }
}

impl Streaming {
    fn poll_streaming(
        &mut self,
        cx: &mut Context<'_>,
    ) -> ControlFlow<StreamItem, BindStreamWithRetryImpl> {
        let error = match self.stream.poll_next_unpin(cx) {
            Poll::Ready(Some(Err(error))) => error,
            poll => return ControlFlow::Break(poll),
        };

        warn!("The bind stream failed: {error}");
        self.info.port_forward.state.lock().status =
            PortForwardStatus::Failed(format!("{error} (will retry)"));

        let now = Instant::now();
        if self.started_at < now - RETRY_COOLDOWN {
            debug!("The bind stream was up for more than {:?}", RETRY_COOLDOWN);
            ControlFlow::Continue(BindStreamWithRetryImpl::StreamingPrep(StreamingPrep {
                started_at: now,
                retry_strategy: default_retry_strategy(),
                stream: Box::pin(self.info.get_bind_stream()),
                info: self.info.clone(),
            }))
        } else {
            debug!("The bind stream crashed before {:?}", RETRY_COOLDOWN);
            let mut retry_strategy = self.retry_strategy.clone();
            let sleep = Box::pin(tokio::time::timeout(
                retry_strategy.delay(),
                self.info.ask.clone(),
            ));
            ControlFlow::Continue(BindStreamWithRetryImpl::Retrying(Retrying {
                sleep,
                retry_strategy,
                info: self.info.clone(),
            }))
        }
    }
}

impl Retrying {
    fn poll_next(
        &mut self,
        cx: &mut Context<'_>,
    ) -> ControlFlow<StreamItem, BindStreamWithRetryImpl> {
        match self.sleep.poll_unpin(cx) {
            Poll::Ready(Err(Elapsed { .. })) => {
                warn!("Retry cooldown: DONE");
                self.info.port_forward.state.lock().status = PortForwardStatus::Pending;
                ControlFlow::Continue(BindStreamWithRetryImpl::StreamingPrep(StreamingPrep {
                    started_at: Instant::now(),
                    retry_strategy: self.retry_strategy.clone(),
                    stream: Box::pin(self.info.get_bind_stream()),
                    info: self.info.clone(),
                }))
            }
            Poll::Ready(Ok(_ask)) => {
                ControlFlow::Break(Poll::Ready(Some(Err(BindError::Canceled.into()))))
            }
            Poll::Pending => {
                warn!("Retry cooldown: PENDING");
                ControlFlow::Break(Poll::Pending)
            }
        }
    }
}

impl StreamInfo {
    fn get_bind_stream(&self) -> impl Future<Output = Result<BindStream, BindError>> + 'static {
        let stream = Box::pin(super::get_bind_stream(
            self.server.clone(),
            self.port_forward.clone(),
            self.ask.clone(),
        ));
        let ask = self.ask.clone();
        async move {
            match futures::future::select(stream, ask).await {
                futures::future::Either::Left((stream, _ask)) => stream,
                futures::future::Either::Right((Ok(()), _stream)) => Err(BindError::Canceled),
                futures::future::Either::Right((Err(oneshot::Canceled), _stream)) => {
                    debug!("Ask to shutdown canceled without being explicitly set");
                    Err(BindError::Canceled)
                }
            }
        }
    }
}

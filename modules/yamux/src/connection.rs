use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::Poll,
};

use futures::future::poll_fn;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::{Mutex as TokioMutex, mpsc, oneshot},
    task::JoinHandle,
};
use tokio_util::compat::TokioAsyncReadCompatExt as _;

use crate::prelude::*;

use super::YamuxStream;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YamuxConnectionType {
    Client,
    Server,
}

impl From<YamuxConnectionType> for yamux::Mode {
    fn from(value: YamuxConnectionType) -> Self {
        match value {
            YamuxConnectionType::Client => yamux::Mode::Client,
            YamuxConnectionType::Server => yamux::Mode::Server,
        }
    }
}

#[derive(Debug, Clone)]
pub struct YamuxConfig {
    pub accept_backlog: usize,
    pub max_stream_window: usize,
    pub max_num_streams: usize,
    pub read_after_close: bool,
    pub split_send_size: usize,
}

impl Default for YamuxConfig {
    fn default() -> Self {
        Self {
            accept_backlog: 100,
            max_stream_window: 1024 * 1024,
            max_num_streams: 256,
            read_after_close: true,
            split_send_size: 16 * 1024,
        }
    }
}

impl From<YamuxConfig> for yamux::Config {
    fn from(config: YamuxConfig) -> Self {
        let mut v = yamux::Config::default();
        const MIN_STREAM_WINDOW: usize = 256 * 1024;
        let per_stream_window = config.max_stream_window.max(MIN_STREAM_WINDOW);
        let connection_window = config.max_num_streams.saturating_mul(per_stream_window);
        v.set_max_connection_receive_window(Some(connection_window));
        v.set_max_num_streams(config.max_num_streams);
        v.set_read_after_close(config.read_after_close);
        v.set_split_send_size(config.split_send_size);
        v
    }
}

pub struct YamuxConnection {
    config: YamuxConfig,
    outbound_tx: mpsc::Sender<oneshot::Sender<Result<YamuxStream>>>,
    inbound_rx: Option<TokioMutex<mpsc::Receiver<YamuxStream>>>,
    stream_count: Arc<AtomicUsize>,
    shutdown_tx: TokioMutex<Option<oneshot::Sender<()>>>,
    driver: TokioMutex<Option<JoinHandle<()>>>,
}

impl YamuxConnection {
    pub fn new<S>(typ: YamuxConnectionType, stream: S, config: YamuxConfig) -> Result<Self>
    where
        S: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    {
        let (outbound_tx, outbound_rx) = mpsc::channel(100);

        let (inbound_tx, inbound_rx) = if config.accept_backlog > 0 {
            let (tx, rx) = mpsc::channel(config.accept_backlog);
            (Some(tx), Some(TokioMutex::new(rx)))
        } else {
            (None, None)
        };

        let stream_count = Arc::new(AtomicUsize::new(0));

        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let connection = yamux::Connection::new(stream.compat(), config.clone().into(), typ.into());

        let driver = ConnectionDriver::new(connection, outbound_rx, inbound_tx, stream_count.clone(), shutdown_rx);
        let driver = tokio::spawn(async move {
            driver.run().await;
        });

        Ok(Self {
            config,
            outbound_tx,
            inbound_rx,
            stream_count,
            shutdown_tx: TokioMutex::new(Some(shutdown_tx)),
            driver: TokioMutex::new(Some(driver)),
        })
    }

    pub fn options(&self) -> &YamuxConfig {
        &self.config
    }

    pub fn stream_count(&self) -> usize {
        self.stream_count.load(Ordering::SeqCst)
    }

    pub async fn connect_stream(&self) -> Result<YamuxStream> {
        let (tx, rx) = oneshot::channel();
        self.outbound_tx
            .send(tx)
            .await
            .map_err(|_| Error::new(ErrorKind::ConnectionClosed).with_message("connection closed"))?;

        match rx.await {
            Ok(result) => result,
            Err(_) => Err(Error::new(ErrorKind::ConnectionClosed).with_message("connection closed")),
        }
    }

    pub async fn accept_stream(&self) -> Result<YamuxStream> {
        let Some(receiver) = &self.inbound_rx else {
            return Err(Error::new(ErrorKind::AcceptDisabled).with_message("accept_stream is disabled"));
        };

        let mut receiver = receiver.lock().await;
        match receiver.recv().await {
            Some(stream) => Ok(stream),
            None => Err(Error::new(ErrorKind::ConnectionClosed).with_message("connection closed")),
        }
    }

    pub async fn close(&self) -> Result<()> {
        if let Some(shutdown_tx) = self.shutdown_tx.lock().await.take() {
            let _ = shutdown_tx.send(());
        }

        if let Some(driver) = self.driver.lock().await.take() {
            match driver.await {
                Ok(()) => {}
                Err(err) => {
                    return Err(Error::from_error(err, ErrorKind::Unknown).with_message("yamux driver join error"));
                }
            }
        }

        Ok(())
    }
}

impl Drop for YamuxConnection {
    fn drop(&mut self) {
        if let Ok(mut shutdown_tx) = self.shutdown_tx.try_lock() {
            if let Some(tx) = shutdown_tx.take() {
                let _ = tx.send(());
            }
        }
        if let Ok(mut driver) = self.driver.try_lock() {
            if let Some(handle) = driver.take() {
                handle.abort();
            }
        }
    }
}

struct ConnectionDriver<S> {
    connection: yamux::Connection<tokio_util::compat::Compat<S>>,
    outbound_rx: mpsc::Receiver<oneshot::Sender<Result<YamuxStream>>>,
    inbound_tx: Option<mpsc::Sender<YamuxStream>>,
    stream_count: Arc<AtomicUsize>,
    shutdown_rx: oneshot::Receiver<()>,
}

impl<S> ConnectionDriver<S>
where
    S: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    fn new(
        connection: yamux::Connection<tokio_util::compat::Compat<S>>,
        outbound_rx: mpsc::Receiver<oneshot::Sender<Result<YamuxStream>>>,
        inbound_tx: Option<mpsc::Sender<YamuxStream>>,
        stream_count: Arc<AtomicUsize>,
        shutdown_rx: oneshot::Receiver<()>,
    ) -> Self {
        Self {
            connection,
            outbound_rx,
            inbound_tx,
            stream_count,
            shutdown_rx,
        }
    }

    async fn run(mut self) {
        let mut outbound_queue: VecDeque<oneshot::Sender<Result<YamuxStream>>> = VecDeque::new();
        let mut closing = false;

        loop {
            tokio::select! {
                _ = &mut self.shutdown_rx, if !closing => {
                    closing = true;
                    self.outbound_rx.close();
                    self.inbound_tx = None;
                    self.fail_pending(&yamux::ConnectionError::Closed, &mut outbound_queue);
                }
                Some(request) = self.outbound_rx.recv(), if !closing => {
                    outbound_queue.push_back(request);
                }
                event = poll_fn(|cx| {
                    if closing {
                        return match self.connection.poll_close(cx) {
                            Poll::Ready(result) => Poll::Ready(ConnectionDriverEvent::Closing(result)),
                            Poll::Pending => Poll::Pending,
                        };
                    }

                    if !outbound_queue.is_empty() {
                        match self.connection.poll_new_outbound(cx) {
                            Poll::Ready(result) => {
                                let request = outbound_queue.pop_front().unwrap();
                                return Poll::Ready(ConnectionDriverEvent::Outbound(result, request))
                            }
                            Poll::Pending => {},
                        }
                    }

                    match self.connection.poll_next_inbound(cx) {
                        Poll::Ready(Some(result)) => Poll::Ready(ConnectionDriverEvent::Inbound(result)),
                        Poll::Ready(None) => Poll::Ready(ConnectionDriverEvent::Closed),
                        Poll::Pending => Poll::Pending,
                    }
                }) => {
                    match event {
                        ConnectionDriverEvent::Inbound(Ok(stream)) => {
                            if !self.handle_inbound(stream) {
                                break;
                            }
                        }
                        ConnectionDriverEvent::Inbound(Err(err)) => {
                            self.fail_pending(&err, &mut outbound_queue);
                            break;
                        }
                        ConnectionDriverEvent::Outbound(result, respond_to) => {
                            if !self.handle_outbound(result, respond_to) {
                                break;
                            }
                        }
                        ConnectionDriverEvent::Closed => {
                            self.fail_pending(&yamux::ConnectionError::Closed, &mut outbound_queue);
                            break;
                        }
                        ConnectionDriverEvent::Closing(result) => {
                            if let Err(err) = result {
                                self.fail_pending(&err, &mut outbound_queue);
                            }
                            break;
                        }
                    }
                }
                else => {
                    break;
                }
            }
        }

        self.fail_pending(&yamux::ConnectionError::Closed, &mut outbound_queue);
    }

    fn handle_inbound(&mut self, stream: yamux::Stream) -> bool {
        let Some(sender) = &self.inbound_tx else {
            return true;
        };

        let yamux_stream = YamuxStream::new(stream, self.stream_count.clone());

        match sender.try_send(yamux_stream) {
            Ok(()) => true,
            Err(mpsc::error::TrySendError::Full(_)) => true,
            Err(mpsc::error::TrySendError::Closed(_)) => false,
        }
    }

    fn handle_outbound(&mut self, result: std::result::Result<yamux::Stream, yamux::ConnectionError>, respond_to: oneshot::Sender<Result<YamuxStream>>) -> bool {
        match result {
            Ok(stream) => {
                let yamux_stream = YamuxStream::new(stream, self.stream_count.clone());
                let _ = respond_to.send(Ok(yamux_stream));
                true
            }
            Err(err) => {
                let is_closed = matches!(err, yamux::ConnectionError::Closed);
                let _ = respond_to.send(Err(err.into()));
                !is_closed
            }
        }
    }

    fn fail_pending(&mut self, err: &yamux::ConnectionError, outbound_queue: &mut VecDeque<oneshot::Sender<Result<YamuxStream>>>) {
        while let Some(request) = outbound_queue.pop_front() {
            let error = match err {
                yamux::ConnectionError::Closed => Error::new(ErrorKind::ConnectionClosed).with_message("connection closed"),
                _ => Error::new(ErrorKind::YamuxError).with_message(err.to_string()),
            };
            let _ = request.send(Err(error));
        }
    }
}

enum ConnectionDriverEvent {
    Inbound(std::result::Result<yamux::Stream, yamux::ConnectionError>),
    Outbound(std::result::Result<yamux::Stream, yamux::ConnectionError>, oneshot::Sender<Result<YamuxStream>>),
    Closing(std::result::Result<(), yamux::ConnectionError>),
    Closed,
}

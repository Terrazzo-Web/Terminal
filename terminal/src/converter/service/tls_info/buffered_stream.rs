use std::io::Result;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::ready;

use pin_project::pin_project;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio::net::TcpStream;

#[pin_project]
pub struct BufferedStream {
    #[pin]
    tcp_stream: TcpStream,
    pub buffer: Vec<u8>,
}

impl From<TcpStream> for BufferedStream {
    fn from(tcp_stream: TcpStream) -> Self {
        Self {
            tcp_stream,
            buffer: vec![],
        }
    }
}

impl AsyncRead for BufferedStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<Result<()>> {
        let this = self.project();
        let start = buf.filled().len();
        let () = ready!(this.tcp_stream.poll_read(cx, buf))?;
        this.buffer.extend(&buf.filled()[start..]);
        Ok(()).into()
    }
}

impl AsyncWrite for BufferedStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        self.project().tcp_stream.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.project().tcp_stream.poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.project().tcp_stream.poll_shutdown(cx)
    }
}

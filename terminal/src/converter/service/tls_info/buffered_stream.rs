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
    pub read_buffer: Vec<u8>,
    pub write_buffer: Vec<u8>,
}

impl From<TcpStream> for BufferedStream {
    fn from(tcp_stream: TcpStream) -> Self {
        Self {
            tcp_stream,
            read_buffer: vec![],
            write_buffer: vec![],
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
        let poll = this.tcp_stream.poll_read(cx, buf);
        this.read_buffer.extend(&buf.filled()[start..]);
        return poll;
    }
}

impl AsyncWrite for BufferedStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        let this = self.project();
        let count = ready!(this.tcp_stream.poll_write(cx, buf))?;
        this.write_buffer.extend(&buf[..count]);
        Ok(count).into()
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.project().tcp_stream.poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.project().tcp_stream.poll_shutdown(cx)
    }
}

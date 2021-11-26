use crate::{inner_ep::InnerEndPoint, EndPointId};
use std::{
    io::{Read, Write},
    pin::Pin,
    sync::Arc,
};
use stream_channel::StreamChannel;
use tokio::io::{self, AsyncRead, AsyncWrite};

pub struct MStreamEndPoint {
    inner: Arc<InnerEndPoint>,
    channel: StreamChannel,
}

impl MStreamEndPoint {
    pub(crate) fn new(inner: Arc<InnerEndPoint>, channel: StreamChannel) -> Self {
        Self { inner, channel }
    }

    pub async fn connect(&mut self, target: EndPointId) -> io::Result<()> {
        self.inner.connect(target).await
    }

    pub async fn accept(&mut self) -> io::Result<()> {
        self.inner.accept().await
    }

    pub async fn close(self) -> io::Result<()> {
        todo!()
    }

    pub fn self_id(&self) -> EndPointId {
        self.inner.self_id()
    }

    pub fn target_id(&self) -> Option<EndPointId> {
        self.inner.target_id()
    }
}

impl Read for MStreamEndPoint {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.channel.read(buf)
    }
}

impl AsyncRead for MStreamEndPoint {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().channel).poll_read(cx, buf)
    }
}

impl Write for MStreamEndPoint {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.channel.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.channel.flush()
    }
}

impl AsyncWrite for MStreamEndPoint {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        _buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        todo!()
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        todo!()
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        todo!()
    }
}

use crate::{inner_ep::InnerEndPoint, EndPointId};
use std::{pin::Pin, sync::Arc};
use stream_channel::async_sc::StreamChannel;
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

    pub async fn read_slice(&mut self) -> io::Result<Box<[u8]>> {
        self.channel.read_slice().await
    }

    pub async fn write_slice(&mut self, data: Box<[u8]>) -> io::Result<()> {
        self.channel.write_slice(data).await
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

impl AsyncWrite for MStreamEndPoint {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        Pin::new(&mut self.get_mut().channel).poll_write(cx, buf)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.get_mut().channel).poll_flush(cx)
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.get_mut().channel).poll_shutdown(cx)
    }
}

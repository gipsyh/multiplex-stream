use crate::{mstream::InnerMStream, EndPointId};
use std::{io, mem, sync::Arc};
use stream_channel::async_sc::StreamChannel;

pub(crate) enum EndPointStatus {
    Unconnected(StreamChannel),
    Connected(EndPointId),
    Disconnected,
}

pub(crate) struct InnerEndPoint {
    mstream: Arc<InnerMStream>,
    self_id: EndPointId,
    pub(crate) status: EndPointStatus,
}

impl InnerEndPoint {
    pub fn new(
        mstream: Arc<InnerMStream>,
        self_id: EndPointId,
        inner_channel: StreamChannel,
    ) -> Self {
        Self {
            self_id,
            status: EndPointStatus::Unconnected(inner_channel),
            mstream,
        }
    }

    pub async fn connect(self: &Arc<Self>, target: EndPointId) -> io::Result<()> {
        if let EndPointStatus::Unconnected(_) = self.status {
            self.mstream.connect(self, target).await
        } else {
            Err(io::ErrorKind::Other.into())
        }
    }

    pub async fn accept(self: &Arc<Self>) -> io::Result<()> {
        if let EndPointStatus::Unconnected(_) = self.status {
            self.mstream.accept(self).await
        } else {
            Err(io::ErrorKind::Other.into())
        }
    }

    pub fn self_id(&self) -> EndPointId {
        self.self_id
    }

    pub fn target_id(&self) -> Option<EndPointId> {
        if let EndPointStatus::Connected(target) = self.status {
            Some(target)
        } else {
            None
        }
    }

    pub fn set_connect(&mut self, target: EndPointId) -> StreamChannel {
        if let EndPointStatus::Unconnected(stream) =
            mem::replace(&mut self.status, EndPointStatus::Connected(target))
        {
            stream
        } else {
            panic!();
        }
    }

    pub fn is_unconnected(&self) -> bool {
        matches!(self.status, EndPointStatus::Unconnected(_))
    }
}

unsafe impl Sync for InnerEndPoint {}

unsafe impl Send for InnerEndPoint {}

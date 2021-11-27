use crate::{
    inner_ep::{EndPointStatus, InnerEndPoint},
    EndPointId, MStreamEndPoint,
};
use async_bincode::{AsyncBincodeStream, AsyncDestination};
use futures::{
    prelude::sink::SinkExt,
    stream::{SplitSink, SplitStream},
};
use futures::{StreamExt};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use stream_channel::StreamChannel;
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    join, spawn,
    sync::{mpsc, oneshot, Mutex},
};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
struct RequestId(usize);

impl RequestId {
    fn new() -> Self {
        Self(rand::thread_rng().gen())
    }
}

#[derive(Serialize, Deserialize)]
enum RequestKind {
    Connect(EndPointId),
    Disconnect,
    Data(Box<[u8]>),
}

#[derive(Serialize, Deserialize)]
struct Request {
    requestid: RequestId,
    self_id: EndPointId,
    kind: RequestKind,
}

impl Request {
    fn response(&self, status: bool) -> Response {
        Response {
            requestid: self.requestid,
            status,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Response {
    requestid: RequestId,
    status: bool,
}

#[derive(Serialize, Deserialize)]
enum Message {
    Request(Request),
    Response(Response),
}

type RequestSender = mpsc::Sender<(
    Request,
    oneshot::Sender<io::Result<bool>>,
    Arc<InnerEndPoint>,
)>;

type RequestReceiver = mpsc::Receiver<(
    Request,
    oneshot::Sender<io::Result<bool>>,
    Arc<InnerEndPoint>,
)>;

type RequestWaitsMap = HashMap<
    RequestId,
    (
        EndPointId,
        oneshot::Sender<io::Result<bool>>,
        Arc<InnerEndPoint>,
    ),
>;

type AcceptWaitsMap = HashMap<EndPointId, (oneshot::Sender<io::Result<()>>, Arc<InnerEndPoint>)>;

type AcceptReceiver = mpsc::Receiver<(oneshot::Sender<io::Result<()>>, Arc<InnerEndPoint>)>;

async fn transfer_write(
    mut stream: StreamChannel,
    inner: Arc<InnerEndPoint>,
    request_sender: RequestSender,
) -> io::Result<()> {
    loop {
        let mut buf = vec![0_u8; 4096];
        let res = stream.read(buf.as_mut_slice()).await?;
        dbg!(res);
        buf.truncate(res);
        let data = buf.into_boxed_slice();
        let request = Request {
            requestid: RequestId::new(),
            self_id: inner.self_id(),
            kind: RequestKind::Data(data),
        };
        let (send, recv) = oneshot::channel();
        request_sender
            .send((request, send, inner.clone()))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::Other, ""))?;
        if !recv
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))??
        {
            return Err(io::Error::new(io::ErrorKind::Other, "transfer data failed"));
        }
    }
}

async fn handle_messgae<SR: AsyncReadExt + Unpin, SW: AsyncWriteExt + Unpin>(
    mut stream_read: SplitStream<AsyncBincodeStream<SR, Message, Message, AsyncDestination>>,
    stream_write: &Mutex<
        SplitSink<AsyncBincodeStream<SW, Message, Message, AsyncDestination>, Message>,
    >,
    response_waits: &Mutex<RequestWaitsMap>,
    accept_waits: &Mutex<AcceptWaitsMap>,
    request_sender: RequestSender,
) -> io::Result<()> {
    loop {
        println!("begin handle message");
        let message = stream_read
            .next()
            .await
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "stream closed"))?
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        match message {
            Message::Request(request) => {
                let response = match request.kind {
                    RequestKind::Connect(target) => match accept_waits.lock().await.remove(&target)
                    {
                        Some((sender, mut inner)) => {
                            assert!(matches!(inner.status, EndPointStatus::Unconnected(_)));
                            let inner_ptr = unsafe { Arc::get_mut_unchecked(&mut inner) };
                            let channel = inner_ptr.set_connect(target);
                            spawn(transfer_write(channel, inner, request_sender.clone()));
                            sender
                                .send(Ok(()))
                                .map_err(|_| io::Error::new(io::ErrorKind::Other, ""))?;
                            request.response(true)
                        }
                        None => request.response(false),
                    },
                    RequestKind::Disconnect => todo!(),
                    RequestKind::Data(_) => todo!(),
                };
                stream_write
                    .lock()
                    .await
                    .send(Message::Response(response))
                    .await
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            }
            Message::Response(response) => {
                let (target, sender, mut inner) = response_waits
                    .lock()
                    .await
                    .remove(&response.requestid)
                    .unwrap();
                if inner.is_unconnected() {
                    let inner_ptr = unsafe { Arc::get_mut_unchecked(&mut inner) };
                    let channel = inner_ptr.set_connect(target);
                    spawn(transfer_write(
                        channel,
                        inner.clone(),
                        request_sender.clone(),
                    ));
                }
                sender.send(Ok(response.status)).unwrap();
            }
        }
    }
}

async fn handle_request<SW: AsyncWriteExt + Unpin>(
    stream_write: &Mutex<
        SplitSink<AsyncBincodeStream<SW, Message, Message, AsyncDestination>, Message>,
    >,
    mut request_receiver: RequestReceiver,
    response_waits: &Mutex<RequestWaitsMap>,
) -> io::Result<()> {
    loop {
        let (request, response, inner) =
            request_receiver.recv().await.ok_or(io::ErrorKind::Other)?;
        let target = if let RequestKind::Connect(target) = request.kind {
            target
        } else {
            inner.target_id().unwrap()
        };
        assert!(response_waits
            .lock()
            .await
            .insert(request.requestid, (target, response, inner))
            .is_none());
        println!("prepare send request to stream");
        stream_write
            .lock()
            .await
            .send(Message::Request(request))
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    }
}

async fn handle_accept(
    mut accpet_receiver: AcceptReceiver,
    accept_waits: &Mutex<AcceptWaitsMap>,
) -> io::Result<()> {
    loop {
        let (response, innerep) = accpet_receiver.recv().await.ok_or(io::ErrorKind::Other)?;
        assert!(accept_waits
            .lock()
            .await
            .insert(innerep.self_id(), (response, innerep))
            .is_none());
    }
}

#[tokio::main]
async fn mstream_main<S: AsyncReadExt + AsyncWriteExt + Unpin>(
    stream: S,
    request_receiver: RequestReceiver,
    request_sender: RequestSender,
    accpet_receiver: mpsc::Receiver<(oneshot::Sender<io::Result<()>>, Arc<InnerEndPoint>)>,
) -> io::Result<()> {
    let response_waits = Mutex::new(HashMap::new());
    let accept_waits = Mutex::new(HashMap::new());
    let stream = AsyncBincodeStream::<_, Message, Message, _>::from(stream).for_async();
    let (stream_write, stream_read) = stream.split();
    let stream_write = Mutex::new(stream_write);
    let handle_messgae = handle_messgae(
        stream_read,
        &stream_write,
        &response_waits,
        &accept_waits,
        request_sender,
    );
    let handle_request = handle_request(&stream_write, request_receiver, &response_waits);
    let handle_accept = handle_accept(accpet_receiver, &accept_waits);
    let (handle_messgae, handle_request, handle_accept) =
        join!(handle_messgae, handle_request, handle_accept);
    handle_messgae.unwrap();
    handle_request.unwrap();
    handle_accept.unwrap();
    todo!();
}

pub(crate) struct InnerMStream {
    request_sender: mpsc::Sender<(
        Request,
        oneshot::Sender<io::Result<bool>>,
        Arc<InnerEndPoint>,
    )>,
    accept_sender: mpsc::Sender<(oneshot::Sender<io::Result<()>>, Arc<InnerEndPoint>)>,
    endpoints_id: Mutex<HashSet<EndPointId>>,
}

impl InnerMStream {
    pub(crate) async fn connect(
        &self,
        self_inner: &Arc<InnerEndPoint>,
        target: EndPointId,
    ) -> io::Result<()> {
        let request = Request {
            requestid: RequestId::new(),
            kind: RequestKind::Connect(target),
            self_id: self_inner.self_id(),
        };
        let (send, recv) = oneshot::channel();
        self.request_sender
            .send((request, send, self_inner.clone()))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::Other, ""))?;
        if recv
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))??
        {
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "connect failed"))
        }
    }

    pub(crate) async fn accept(&self, self_inner: &Arc<InnerEndPoint>) -> io::Result<()> {
        let (send, recv) = oneshot::channel();
        self.accept_sender
            .send((send, self_inner.clone()))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::Other, ""))?;
        recv.await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
    }
}

pub struct MStream {
    inner: Arc<InnerMStream>,
    // handle: std::thread::JoinHandle<dyn futures::Future<Output = io::Result<()>>>,
}

impl MStream {
    pub fn new<S: AsyncReadExt + AsyncWriteExt + Unpin + Send + 'static>(stream: S) -> Self {
        let (request_sender, request_receiver) = mpsc::channel(1024);
        let (accept_sender, accpet_receiver) = mpsc::channel(1024);
        let request_sender_clone = request_sender.clone();
        let _handle = std::thread::spawn(|| {
            mstream_main(
                stream,
                request_receiver,
                request_sender_clone,
                accpet_receiver,
            )
        });
        Self {
            // handle,
            inner: Arc::new(InnerMStream {
                request_sender,
                endpoints_id: Mutex::new(HashSet::new()),
                accept_sender,
            }),
        }
    }

    pub async fn new_endpoint(&self, self_id: EndPointId) -> io::Result<MStreamEndPoint> {
        if self.inner.endpoints_id.lock().await.contains(&self_id) {
            return Err(io::ErrorKind::Other.into());
        } else {
            self.inner.endpoints_id.lock().await.insert(self_id);
        }
        let (innerchannel, outerchannel) = StreamChannel::new();
        let inner = Arc::new(InnerEndPoint::new(
            self.inner.clone(),
            self_id,
            innerchannel,
        ));
        Ok(MStreamEndPoint::new(inner, outerchannel))
    }
}

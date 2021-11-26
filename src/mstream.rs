use crate::{
    inner_ep::{EndPointStatus, InnerEndPoint},
    EndPointId, MStreamEndPoint,
};
use async_bincode::{AsyncBincodeStream, AsyncDestination};
use futures::prelude::sink::SinkExt;
use futures::StreamExt;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use stream_channel::StreamChannel;
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    sync::{mpsc, oneshot, Mutex},
};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
struct RequestId(usize);

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

async fn handle_messgae<S: AsyncReadExt + AsyncWriteExt + Unpin>(
    message: Message,
    stream: &mut AsyncBincodeStream<S, Message, Message, AsyncDestination>,
    response_waits: &mut HashMap<
        RequestId,
        (
            EndPointId,
            oneshot::Sender<io::Result<bool>>,
            Arc<InnerEndPoint>,
        ),
    >,
    accept_waits: &mut HashMap<EndPointId, (oneshot::Sender<io::Result<()>>, Arc<InnerEndPoint>)>,
) -> io::Result<()> {
    match message {
        Message::Request(request) => {
            let response = match request.kind {
                RequestKind::Connect(target) => match accept_waits.remove(&target) {
                    Some((sender, mut inner)) => {
                        assert!(matches!(inner.status, EndPointStatus::Unconnected(_)));
                        let inner_ptr = unsafe { Arc::get_mut_unchecked(&mut inner) };
                        let channel = inner_ptr.set_connect(target);
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
            stream
                .send(Message::Response(response))
                .await
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        }
        Message::Response(response) => {
            let (target, sender, mut inner) = response_waits.remove(&response.requestid).unwrap();
            if inner.is_unconnected() {
                let inner_ptr = unsafe { Arc::get_mut_unchecked(&mut inner) };
                let channel = inner_ptr.set_connect(target);
            }
            sender.send(Ok(response.status)).unwrap();
        }
    }
    Ok(())
}

async fn dispatch_message<S: AsyncReadExt + AsyncWriteExt + Unpin>(
    stream: S,
    mut request_receiver: mpsc::Receiver<(
        Request,
        oneshot::Sender<io::Result<bool>>,
        Arc<InnerEndPoint>,
    )>,
    mut accpet_receiver: mpsc::Receiver<(oneshot::Sender<io::Result<()>>, Arc<InnerEndPoint>)>,
) -> io::Result<()> {
    let mut response_waits = HashMap::new();
    let mut accept_waits = HashMap::new();
    let mut stream = AsyncBincodeStream::<_, Message, Message, _>::from(stream).for_async();
    loop {
        tokio::select! {
            res = stream.next() => {
                let message = res.unwrap().map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                handle_messgae(message, &mut stream, &mut response_waits, &mut accept_waits).await.unwrap();
            },
            res = request_receiver.recv() => {
                let (request, response, inner) = res.ok_or(io::ErrorKind::Other)?;
                let target = if let RequestKind::Connect(target) = request.kind {
                    target
                } else {
                    inner.target_id().unwrap()
                };
                assert!(response_waits
                    .insert(request.requestid, (target, response, inner))
                    .is_none());
                stream
                    .send(Message::Request(request))
                    .await
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            },
            res = accpet_receiver.recv() => {
                let (response, innerep) = res.ok_or(io::ErrorKind::Other)?;
                assert!(accept_waits
                    .insert(innerep.self_id(), (response, innerep))
                    .is_none());
            }
        }
        // {
        //     let (request, response, inner) =
        //         request_receiver.recv().await.ok_or(io::ErrorKind::Other)?;
        //     let target = if let RequestKind::Connect(target) = request.kind {
        //         target
        //     } else {
        //         inner.target_id().unwrap()
        //     };
        //     assert!(response_waits
        //         .insert(request.requestid, (target, response, inner))
        //         .is_none());
        //     stream
        //         .send(Message::Request(request))
        //         .await
        //         .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        // }
        // {
        //     let (response, innerep) = accpet_receiver.recv().await.ok_or(io::ErrorKind::Other)?;
        //     assert!(accept_waits
        //         .insert(innerep.self_id(), (response, innerep))
        //         .is_none());
        // }
        //             let mut endpoints = endpoints.lock().await;
        //             let endpoint = endpoints.get_mut(&header.target).unwrap();
        //             endpoint.write.write_data_in_vec(data).unwrap();
    }
}

// handle: JoinHandle<impl Future<Output = Result<(), Error>>>,

// async fn transfer_write(
//     mut endpoint: InnerEndPointRead,
// ) -> io::Result<(Vec<u8>, InnerEndPointRead)> {
//     let mut buf = vec![0_u8; 4096];
//     let res = endpoint.read.read(buf.as_mut_slice()).await?;
//     dbg!(res);
//     buf.truncate(res);
//     let header = MStreamRequest {
//         endpid: endpoint.innerep.target_id(),
//         length: res,
//     };
//     let mut ans = bincode::serialize(&header).unwrap();
//     ans.extend(buf);
//     Ok((ans, endpoint))
// }

// async fn undispatch_write<S: AsyncWriteExt + Unpin + Send + 'static>(
//     mut stream: S,
//     endpoints: Vec<InnerEndPointRead>,
// ) -> io::Result<()> {
//     let mut ans = Vec::new();
//     for endpoint in endpoints {
//         ans.push(spawn(transfer_write(endpoint)));
//     }
//     loop {
//         let ret;
//         let _tmp;
//         (ret, _tmp, ans) = futures::future::select_all(ans).await;
//         let (data, endpoint) = ret.unwrap().unwrap();
//         stream.write_all(data.as_slice()).await?;
//         ans.push(spawn(transfer_write(endpoint)));
//     }
// }

#[tokio::main]
async fn mstream_main<S: AsyncReadExt + AsyncWriteExt + Unpin + Send + 'static>(
    stream: S,
    request_receiver: mpsc::Receiver<(
        Request,
        oneshot::Sender<io::Result<bool>>,
        Arc<InnerEndPoint>,
    )>,
    accpet_receiver: mpsc::Receiver<(oneshot::Sender<io::Result<()>>, Arc<InnerEndPoint>)>,
) -> io::Result<()> {
    tokio::join!(dispatch_message(stream, request_receiver, accpet_receiver)).0
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
            requestid: RequestId(rand::thread_rng().gen()),
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
        let _handle =
            std::thread::spawn(|| mstream_main(stream, request_receiver, accpet_receiver));
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

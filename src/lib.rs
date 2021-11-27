#![feature(destructuring_assignment)]
#![feature(get_mut_unchecked)]

mod inner_ep;
mod mstream;
mod outer_ep;

pub use mstream::*;
pub use outer_ep::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EndPointId(pub usize);

#[cfg(test)]
mod tests {
    use crate::{mstream::MStream, EndPointId};
    use std::time::Duration;
    use tokio::{
        io::AsyncWriteExt,
        net::{TcpListener, TcpStream},
    };

    #[tokio::test]
    async fn server() {
        let (stream, _) = TcpListener::bind("127.0.0.1:9999")
            .await
            .unwrap()
            .accept()
            .await
            .unwrap();
        let mstream = MStream::new(stream);
        let mut endp = mstream.new_endpoint(EndPointId(1)).await.unwrap();
        endp.accept().await.unwrap();
        println!("connect done");
        let buf = vec![1, 2];
        endp.write_slice(buf.into_boxed_slice()).await.unwrap();
        endp.flush().await.unwrap();
        println!("done");
        loop {}
    }

    #[tokio::test]
    async fn client() {
        let stream = TcpStream::connect("127.0.0.1:9999").await.unwrap();
        let mstream = MStream::new(stream);
        let mut endp = mstream.new_endpoint(EndPointId(1)).await.unwrap();
        std::thread::sleep(Duration::from_secs(1));
        while endp.connect(EndPointId(1)).await.is_err() {}
        println!("connect done");
        // dbg!(endp.connect(EndPointId(1)).await);
        // let mut buf = vec![0, 0];
        // endp.read_exact(buf.as_mut_slice()).await.unwrap();
        // dbg!(buf);
        println!("done");
        loop {}
    }
}

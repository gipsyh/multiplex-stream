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
    use tokio::{
        net::{TcpListener, TcpStream},
    };

    use crate::{mstream::MStream, EndPointId};

    #[tokio::test]
    async fn aysnc_server() {
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
        std::io::Write::write(&mut endp, &buf).unwrap();
        std::io::Write::flush(&mut endp).unwrap();
        println!("done");
        loop {}
    }

    #[test]
    fn server() {
        std::thread::spawn(|| aysnc_server()).join();
    }

    #[tokio::test]
    async fn aysnc_client() {
        let stream = TcpStream::connect("127.0.0.1:9999").await.unwrap();
        let mstream = MStream::new(stream);
        let mut endp = mstream.new_endpoint(EndPointId(1)).await.unwrap();
        while endp.connect(EndPointId(1)).await.is_err() {}
        println!("connect done");
        // dbg!(endp.connect(EndPointId(1)).await);
        // let mut buf = vec![0, 0];
        // endp.read_exact(buf.as_mut_slice()).await.unwrap();
        // dbg!(buf);
        println!("done");
        loop {}
    }

    #[test]
    fn client() {
        std::thread::spawn(|| aysnc_client()).join();
    }
}

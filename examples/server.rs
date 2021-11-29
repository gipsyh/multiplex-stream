use multiplex_stream::{EndPointId, MStream};
use tokio::{io::AsyncWriteExt, join, net::TcpListener};
#[tokio::main]
async fn main() {
    let (stream, _) = TcpListener::bind("127.0.0.1:9999")
        .await
        .unwrap()
        .accept()
        .await
        .unwrap();
    let mstream = MStream::new(stream);
    let mut endp1 = mstream.new_endpoint(EndPointId(1)).await.unwrap();
    let mut endp2 = mstream.new_endpoint(EndPointId(2)).await.unwrap();
    let f1 = async {
        endp1.accept().await.unwrap();
        let buf = vec![1, 2];
        endp1.write_slice(buf.into_boxed_slice()).await.unwrap();
        endp1.flush().await.unwrap();
    };
    let f2 = async {
        endp2.accept().await.unwrap();
        let buf = Vec::from(endp2.read_slice().await.unwrap());
        assert_eq!(buf, vec![3, 4]);
    };
    join!(f1, f2);
    println!("server done");
    loop {}
}

use multiplex_stream::{EndPointId, MStream};
use tokio::{io::AsyncWriteExt, net::TcpListener};
#[tokio::main]
async fn main() {
    let (stream, _) = TcpListener::bind("127.0.0.1:9999")
        .await
        .unwrap()
        .accept()
        .await
        .unwrap();
    let mstream = MStream::new(stream);
    let mut endp = mstream.new_endpoint(EndPointId(1)).await.unwrap();
    endp.accept().await.unwrap();
    let buf = vec![1, 2];
    endp.write_slice(buf.into_boxed_slice()).await.unwrap();
    endp.flush().await.unwrap();
    println!("connect done");
    loop {}
}

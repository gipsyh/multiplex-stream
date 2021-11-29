use multiplex_stream::{EndPointId, MStream};
use std::time::Duration;
use tokio::net::TcpStream;

#[tokio::main]
async fn main() {
    let stream = TcpStream::connect("127.0.0.1:9999").await.unwrap();
    let mstream = MStream::new(stream);
    let mut endp1 = mstream.new_endpoint(EndPointId(1)).await.unwrap();
    let mut endp2 = mstream.new_endpoint(EndPointId(2)).await.unwrap();
    std::thread::sleep(Duration::from_secs(1));
    endp1.connect(EndPointId(1)).await.unwrap();
    endp2.connect(EndPointId(2)).await.unwrap();
    let data = Vec::from(endp1.read_slice().await.unwrap());
    assert_eq!(data, vec![1, 2]);
    let data = vec![3, 4].into_boxed_slice();
    endp2.write_slice(data).await.unwrap();
    println!("client done");
    loop {}
}

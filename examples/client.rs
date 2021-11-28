use multiplex_stream::{EndPointId, MStream};
use std::time::Duration;
use tokio::net::TcpStream;

#[tokio::main]
async fn main() {
    let stream = TcpStream::connect("127.0.0.1:9999").await.unwrap();
    let mstream = MStream::new(stream);
    let mut endp = mstream.new_endpoint(EndPointId(1)).await.unwrap();
    std::thread::sleep(Duration::from_secs(1));
    endp.connect(EndPointId(1)).await.unwrap();
    loop {}
}

use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt};

#[tokio::main]
async fn main() {
    let server_addr = "127.0.0.1:8080";

    match TcpStream::connect(server_addr).await {
        Ok(mut stream) => {
            println!("Connected to the server on {}", server_addr);

            let mut buffer = [0; 1024];
            match stream.read(&mut buffer).await {
                Ok(n) => {
                    println!("Received data from the server: {:?}", &buffer[..n]);
                }
                Err(e) => {
                    eprintln!("Failed to read from socket: {}", e);
                    return;
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to connect to the server: {}", e);
        }
    }
}

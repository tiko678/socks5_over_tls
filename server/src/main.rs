use tokio::net::{TcpListener, TcpStream};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use native_tls::Identity;
use tokio_native_tls::{TlsAcceptor, TlsStream};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let cert_and_key = include_bytes!("/path/ssl.pfx");
    let identity = Identity::from_pkcs12(cert_and_key, "123456").expect("Failed to create Identity");
    println!("Server listening on 127.0.0.1:8000...");
    let native_acceptor = native_tls::TlsAcceptor::new(identity)?;
    let acceptor = TlsAcceptor::from(native_acceptor);
    let listener = TcpListener::bind("0.0.0.0:8000").await?;

    while let Ok((stream, _)) = listener.accept().await {
        let acceptor = acceptor.clone();
        tokio::spawn(async move {
            match acceptor.accept(stream).await {
                Ok(tls_stream) => {
                    if let Err(e) = handle_client(tls_stream).await {
                        println!("Error handling client: {}", e);
                    }
                }
                Err(e) => println!("Failed to accept TLS connection: {}", e),
            }
        });
    }
    Ok(())
}

async fn handle_client(mut client_stream: TlsStream<TcpStream>) -> Result<()> {    
    let mut buffer = [0u8; 4096];
    let n = client_stream.read(&mut buffer).await.expect("Failed to read from client");
    //print!("{}", String::from_utf8_lossy(&buffer[..n]));
    
    if buffer[0] == 5 && buffer[1] > 0 {
        // Send a negotiation response and select a method that does not require authentication
        client_stream.write_all(&[5,0]).await.expect("Failed to write to client");
        // Read client request details
        let n = client_stream.read(&mut buffer).await.expect("Failed to read from client");
        // Handling CONNECT requests
        if buffer[0] == 5 && buffer[1] == 1 {
            // Get target address and port
            let (target_addr, target_port) = parse_target_address(&buffer[3..n]);
            // Connect to target server
            let target_stream = TcpStream::connect((target_addr, target_port)).await.expect("Failed to connect to target");
            // Send a successful connection response to the client
            client_stream.write_all(&[5, 0, 0, 1, 0, 0, 0, 0, 0, 0]).await.expect("Failed to write to client");
            // Start bidirectional data forwarding
            tokio::spawn(forward_data(client_stream, target_stream));
        }
    }
    Ok(())
}

fn parse_target_address(data: &[u8]) -> (String, u16) {
    let atyp = data[0];
    match atyp {
        1 => {
            // IPv4 
            let ip = format!("{}.{}.{}.{}", data[1], data[2], data[3], data[4]);
            let port = u16::from_be_bytes([data[5], data[6]]);
            (ip, port)
        }
        3 => {
            // domain & address
            let len = data[1] as usize;
            let domain = String::from_utf8_lossy(&data[2..2 + len]);
            let port = u16::from_be_bytes([data[2 + len], data[2 + len + 1]]);
            (domain.into_owned(), port)
        }
        4 => {
            // IPv6 
            // Note: This needs to be parsed according to the format of the IPv6 address
            unimplemented!("IPv6 address not implemented")
        }
        _ => unimplemented!("Unsupported address type: {}", atyp),
    }
}


async fn forward_data(source: TlsStream<TcpStream>, mut destination: TcpStream) {
    let (mut source_reader, mut source_writer) = io::split(source);
    let (mut destination_reader, mut destination_writer) = destination.split();
    println!("Entery forward_data");

    let client_to_target = async {
        let mut buffer = [0u8; 4096];
        loop {
            match source_reader.read(&mut buffer).await {
                Ok(0) => break,
                Ok(n) => {
                    //println!("Client data:: {:?}", String::from_utf8_lossy(&buffer[..n]));
                    if destination_writer.write_all(&buffer[..n]).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    };

    let target_to_client = async {
        let mut buffer = [0u8; 4096];
        loop {
            match destination_reader.read(&mut buffer).await {
                Ok(0) => break,
                Ok(n) => {
                    //println!("Server data:: {:?}", String::from_utf8_lossy(&buffer[..n]));
                    if source_writer.write_all(&buffer[..n]).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    };

    tokio::select! {
        _ = client_to_target => {},
        _ = target_to_client => {},
    }
}



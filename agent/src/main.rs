use tokio::net::{TcpListener, TcpStream};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use native_tls::TlsConnector as NativeTlsConnector;
use tokio_native_tls::{TlsConnector, TlsStream};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    println!("Listening on 0.0.0.0:8080...");

    while let Ok((client_stream, _)) = listener.accept().await {
        tokio::spawn(handle_client(client_stream));
    }

    Ok(())
}

async fn handle_client(mut client_stream: TcpStream) -> Result<()> {
    let mut buffer = [0u8; 4096];
    let n = client_stream.read(&mut buffer).await?;

    if buffer[0] == 5 {
        let connector = NativeTlsConnector::new()?;
        let connector = TlsConnector::from(connector);
        let tcp_stream = TcpStream::connect("example.com:8000").await?;
        let mut tls_stream = connector.connect("example.com", tcp_stream).await?;
        tls_stream.write_all(&buffer[..n]).await?;

        let mut response = vec![0; 4096];
        let n = tls_stream.read(&mut response).await?;
        client_stream.write_all(&response[..n]).await?;

        tokio::spawn(forward_data(client_stream, tls_stream));
    }

    Ok(())
}

async fn forward_data(mut source: TcpStream,destination: TlsStream<TcpStream>) -> Result<()> {
    let (mut source_reader, mut source_writer) = source.split();
    let (mut dest_reader, mut dest_writer) = io::split(destination);

    let client_to_target = async {
        let mut buffer = [0u8; 4096];
        loop {
            let n = source_reader.read(&mut buffer).await?;
            if n == 0 { break; }
            dest_writer.write_all(&buffer[..n]).await?;
        }
        Result::<()>::Ok(())
    };

    let target_to_client = async {
        let mut buffer = [0u8; 4096];
        loop {
            let n = dest_reader.read(&mut buffer).await?;
            if n == 0 { break; }
            source_writer.write_all(&buffer[..n]).await?;
        }
        Result::<()>::Ok(())
    };

    tokio::select! {
        _ = client_to_target => {},
        _ = target_to_client => {},
    }

    Ok(())
}


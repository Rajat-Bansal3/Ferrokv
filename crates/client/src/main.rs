use proto;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:6379").await?;

    stream.write_all(b"*1\r\n$4\r\nPING\r\n").await?;

    stream
        .write_all(b"*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n")
        .await?;

    stream
        .write_all(b"*2\r\n$3\r\nGET\r\n$3\r\nfoo\r\n")
        .await?;

    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).await?;
    println!("{}", std::str::from_utf8(&buf[..n])?);
    let mut parser = proto::Parser::new(std::str::from_utf8(&buf[..n]).unwrap().as_bytes());
    loop {
        match parser.parse() {
            Ok(Some(value)) => println!("{:?}", value),
            Ok(None) => break,
            Err(e) => {
                println!("parse error: {:?}", e);
                break;
            }
        }
    }

    Ok(())
}

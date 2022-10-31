use crate::options::ContainerOptions;
use std::io::{Error, Read};
use std::net::TcpListener;
use std::sync::Arc;

pub fn new_listener(options: &ContainerOptions) -> Result<Arc<TcpListener>, Error> {
    let addr = format!("0.0.0.0:{}", options.port);
    let listener = TcpListener::bind(addr)?;

    Ok(Arc::new(listener))
}

pub fn read_push(listener: Arc<TcpListener>) {
    loop {
        match parse_stream(listener.clone()) {
            Ok(value) => {
                if value {
                    return;
                } else {
                    println!("Recieved stream. But it was not valid push req");
                }
            }
            Err(err) => {
                println!("Failed to read incoming stream. Err: {err}");
            }
        }
    }
}

fn parse_stream(listener: Arc<TcpListener>) -> Result<bool, Box<dyn std::error::Error>> {
    let (mut stream, _) = listener.accept()?;

    let mut final_buffer = vec![];
    loop {
        let mut buf = vec![0u8; 1000];
        let read = stream.read(&mut buf)?;

        buf.shrink_to(read);
        final_buffer.append(&mut buf);

        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);

        let status = match req.parse(&final_buffer[..]) {
            Ok(status) => status,
            Err(_) => return Ok(false),
        };
        if status.is_complete() {
            println!("{req:?}");

            if req.method == Some("POST") {
                return Ok(true);
            } else {
                return Ok(false);
            }
        }
    }
}

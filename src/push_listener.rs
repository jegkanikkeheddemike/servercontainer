use httparse::Header;

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
        let mut buf = [0u8; 10];
        let read = stream.read(&mut buf)?;

        final_buffer.extend_from_slice(&buf[0..read]);

        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);

        let status = match req.parse(&final_buffer[..]) {
            Ok(status) => status,
            Err(_) => return Ok(false),
        };
        if status.is_complete() {
            if req.method == Some("POST") {
                break;
            } else {
                return Ok(false);
            }
        }
    }

    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut request = httparse::Request::new(&mut headers);

    request.parse(&final_buffer[..])?;

    let mut found_header = false;
    let mut content_length: usize = 0;
    for header in request.headers {
        if header.name == "Content-Length" {
            found_header = true;
            content_length = String::from_utf8(header.value.to_vec())?.parse()?;
            break;
        }
    }
    if found_header {
        let str = String::from_utf8(final_buffer.clone())?;

        let body_start = match str.find("\r\n\r\n") {
            Some(index) => index,
            None => {
                return Ok(false);
            }
        };
        let mut body = String::from(str.split_at(body_start+4).1);

        if body.len() != content_length {
            let mut rest_of_body_buffer = vec![0u8; content_length - body.len()];
            stream.read_exact(&mut rest_of_body_buffer)?;

            body = format!("{body}{}", String::from_utf8(rest_of_body_buffer)?);
        }

        println!("body: \n{body}");

        std::fs::write("./body_log", &body)?;

        Ok(true)
    } else {
        Ok(false)
    }
}

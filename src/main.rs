use clap::Parser;
use tokio::io;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    socket_address: String,
}

pub fn bodiless_request(line: &String) -> bool {
    let bodiless_http_requests: Vec<&str> = vec!["CONNECT", "GET", "HEAD", "OPTIONS", "TRACE"];

    for no_body in bodiless_http_requests.clone().into_iter() {
        if line.starts_with(no_body) {
            return true;
        }
    }
    false
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let args = Args::parse();

    let result = match serve(&args.socket_address).await {
        Ok(v) => v,
        Err(e) => panic!("{}", e),
    };

    Ok(result)
}

async fn serve(socket_address: &str) -> io::Result<()> {
    let header_split = "\r\n\r\n";
    let external_client = TcpListener::bind(socket_address).await?;

    loop {
        let (mut external_client, _) = external_client.accept().await?;

        tokio::spawn(async move {
            let mut headers = String::from("");
            let mut header_end = 0;
            let mut header_lines: Vec<&str> = vec![];
            let mut body_length = 0;

            let mut request: String = "".to_string();
            /*
                https://www.rfc-editor.org/rfc/rfc7230#section-3
                HTTP-message = start-line
                               *( header-field CRLF )
                               CRLF
                               [ message-body ]
            */
            let (mut rd_external_client, mut wr_external_client) = external_client.split();

            'outer: loop {
                let mut rd_external_client_buffer = vec![0; 512];
                let n = match rd_external_client
                    .read(&mut rd_external_client_buffer)
                    .await
                {
                    Ok(n) => n,
                    Err(e) => panic!("{}", e),
                };

                if n == 0 {
                    break 'outer;
                }

                match String::from_utf8(rd_external_client_buffer[0..n].to_vec()) {
                    Ok(v) => request += &v,
                    Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
                };

                if header_lines.is_empty() {
                    let mut at = header_end;
                    while at < request.len() - header_split.len() + 1 {
                        if request.chars().nth(at).unwrap() == '\r'
                            && request.chars().nth(at + 1).unwrap() == '\n'
                            && request.chars().nth(at + 2).unwrap() == '\r'
                            && request.chars().nth(at + 3).unwrap() == '\n'
                        {
                            headers = request[0..at].to_owned();
                            header_lines = headers.split("\r\n").collect();

                            for header in &header_lines {
                                if header.starts_with("Content-Length:") {
                                    let parts: Vec<&str> = header.split(":").collect();
                                    let num = parts.last().unwrap();
                                    body_length = match num.trim().parse::<usize>() {
                                        Ok(i) => i,
                                        Err(_e) => 0,
                                    };
                                }
                            }
                        }
                        at += 1;
                    }
                    header_end = at;
                }

                if header_lines.len() == 0 {
                    // wait until headers are fully received
                    continue;
                }

                if bodiless_request(&headers) && request.ends_with(header_split) {
                    // GET requests end with "\r\n\r\n"
                    break 'outer;
                }
                if headers.len() + header_split.len() + body_length <= request.len() {
                    // POST body has been fully received
                    break 'outer;
                }
            }

            // println!("......");
            // println!("{}", request);

            let content = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                request.len(),
                request
            );
            wr_external_client.write_all(content.as_bytes()).await?;

            let _ = wr_external_client.shutdown();

            Ok::<_, io::Error>(())
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{bodiless_request, serve};
    use std::str;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;
    use tokio::time::sleep;

    #[test]
    fn test_bodiless_request() {
        assert_eq!(bodiless_request(&"GET / HTTP/1.1".to_owned()), true);
        assert_eq!(
            bodiless_request(&"POST / HTTP/1.1\r\n\r\nBODY".to_owned()),
            false
        );
        assert_eq!(
            bodiless_request(&"PUT / HTTP/1.1\r\nContent-Type: total/funk\r\n\r\nBODY".to_owned()),
            false
        );
    }

    #[tokio::test]
    async fn can_proxy_get_requests() {
        let socket_address = "127.0.0.1:33333";

        tokio::spawn(async move {
            let result = serve(socket_address).await.unwrap();
            assert_eq!(result, ());
        });
        sleep(Duration::from_millis(10)).await; // wait for proxy to be available

        // basic GET request
        let mut proxy = match TcpStream::connect(&socket_address).await {
            Ok(proxy) => proxy,
            Err(err) => panic!("{}", err),
        };
        let message_in = "GET TEST\r\nOther: other\r\n\r\n";
        let wl = proxy.write(message_in.as_bytes()).await.unwrap();
        assert_ne!(wl, 0);

        let read_buffer: &mut [u8] = &mut [0; 500];
        let rl = proxy.read(read_buffer).await.unwrap();
        let message_out = str::from_utf8(&read_buffer[0..rl]).unwrap();

        let expect = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 26\r\n\r\nGET TEST\r\nOther: other\r\n\r\n";
        assert_eq!(expect, message_out);

        // basic POST request
        let mut proxy = match TcpStream::connect(&socket_address).await {
            Ok(proxy) => proxy,
            Err(err) => panic!("{}", err),
        };
        let message_in = "POST TEST\r\nContent-Length: 4\r\n\r\nBODY";
        let wl = proxy.write(message_in.as_bytes()).await.unwrap();
        assert_ne!(wl, 0);

        let read_buffer: &mut [u8] = &mut [0; 500];
        let rl = proxy.read(read_buffer).await.unwrap();
        let message_out = str::from_utf8(&read_buffer[0..rl]).unwrap();

        let expect = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 36\r\n\r\nPOST TEST\r\nContent-Length: 4\r\n\r\nBODY";
        assert_eq!(expect, message_out);
    }
}

extern crate collections;

use std::io::{BufferedReader, TcpListener, Listener, Acceptor};
use std::io::net::ip::{SocketAddr};
use std::from_str::{from_str};
use std::to_str::{ToStr};
use std::str::from_utf8;
use collections::hashmap::HashMap;

pub type Headers = HashMap<~str, ~str>;
pub type SCGIMessage = (Headers, ~[u8]);

pub struct SCGIServer {
    listen_address: SocketAddr,
    // XXX: if i write Sender<SCGIMessage, Sender<SCGIMessage>>, that expands to ((Header, ~[u8]), SCGIMessage), and not (Header, ~[u8], SCGIMessage), sad
    handler: Sender<(Headers, ~[u8], Sender<SCGIMessage>)>
}

impl SCGIServer {
    pub fn new(addr: SocketAddr, handler: Sender<(Headers, ~[u8], Sender<SCGIMessage>)>) -> SCGIServer {
        SCGIServer {
            listen_address: addr,
            handler: handler
        }
    }

    pub fn start(&self) {
        let mut listener = match TcpListener::bind(self.listen_address) { 
            Ok(listener) => listener, 
            Err(_) => fail!("Unable to bind to address {:s}", self.listen_address.to_str()) 
        };
        let mut acceptor = match listener.listen() {
            Ok(acceptor) => acceptor,
            Err(err) => fail!("Cannot listen on the bound.\nError: {:s}", err.to_str())
        };
        for stream in acceptor.incoming() {
            let sender = self.handler.clone();
            spawn(proc() {
                let mut stream = stream.clone();
                let mut reader = BufferedReader::new(stream.clone());
                // XXX: ohgodwhat
                let headers_length = 
                    reader.read_until(58).map(|b| from_str::<uint>(from_utf8(b.init()).expect("Unable to parse headers' length")).expect("Unable to parse headers' length")).unwrap();
                let mut headers_read = 0;
                let mut headers = HashMap::new();

                loop {
                    if headers_read >= headers_length {
                        break;
                    }
                    
                    let header_name = reader.read_until(0).map(|b| from_utf8(b.init()).expect("Invalid header name string").to_owned()).unwrap();
                    let header_value = reader.read_until(0).map(|b| from_utf8(b.init()).expect("Invalid header value string").to_owned()).unwrap();
                    headers_read += header_name.len() + header_value.len() + 2;
                    headers.insert(header_name, header_value);
                }
                // Next character is a comma, after that it's the body
                assert!(reader.read_byte().unwrap() == 44);
                assert!(headers.contains_key(&~"SCGI") && headers.contains_key(&~"CONTENT_LENGTH"));
                let body = reader.read_bytes(from_str::<uint>(*headers.get(&~"CONTENT_LENGTH")).expect("CONTENT_LENGTH is not a number")).unwrap();

                let (response_sender, response_receiver) = channel();
                sender.send((headers, body, response_sender));

                let (response_headers, response_body) = response_receiver.recv();

                for (key, value) in response_headers.iter() {
                    stream.write(format!("{:s}: {:s}\r\n", *key, *value).as_bytes());
                }
                stream.write(bytes!("\r\n"));
                stream.write(response_body);
            });
        }
    }
}
extern crate collections;

use scgi::SCGIServer;
use collections::hashmap::HashMap;

mod scgi;

fn main() {
    let (sender, receiver) = channel();
    let server = SCGIServer::new(from_str("127.0.0.1:9001").unwrap(), sender);
    spawn(proc() { server.start(); });
    let mut default_response_headers = HashMap::new();
    default_response_headers.insert(~"X-Gateway", ~"scgi.rs");
    default_response_headers.insert(~"Status", ~"200 OK");
    loop {
        let (headers, body, response_sender) = receiver.recv();
        let notfound = { 
            let mut response_headers = default_response_headers.clone();
            response_headers.insert_or_update_with(~"Status", ~"404 Missing", |x, y| {});
            response_sender.send((response_headers, bytes!("Not found").to_owned()));
        };
        match headers.find(&~"DOCUMENT_URI").map(|x| x.as_slice()) {
            Some("/") => {
                response_sender.send((default_response_headers.clone(), format!("Hello world!").as_bytes().to_owned()));
            }
            Some(_) => notfound,
            None => notfound
        }
    }
}

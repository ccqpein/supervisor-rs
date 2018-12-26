use std::env;
use std::io::prelude::*;
use std::net::TcpStream;

fn main() {
    let arguments = env::args();
    let change_2_vec = arguments.collect::<Vec<String>>();

    if change_2_vec.len() > 3 {
        println!("{}", "too much arguments, not support yet.");
        return;
    }

    let data_2_server = change_2_vec[1..].join(" ");

    let mut stream = TcpStream::connect("127.0.0.1:33889").unwrap();
    stream.write_all(data_2_server.as_bytes()).unwrap();
    stream.flush().unwrap();
}

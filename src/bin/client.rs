use std::env;
use std::io::prelude::*;
use std::net::TcpStream;
use supervisor_rs::client::Command;

fn main() {
    let arguments = env::args();
    let change_2_vec = arguments.collect::<Vec<String>>();
    let cache_command = Command::new_from_string(change_2_vec[1..].to_vec());

    let mut stream = if let Some(_) = cache_command.prep {
        TcpStream::connect(format!(
            "{}{}",
            cache_command.obj.unwrap().as_str(),
            ":33889"
        ))
        .unwrap()
    } else {
        TcpStream::connect("127.0.0.1:33889").unwrap()
    };

    let data_2_server = format!(
        "{} {}",
        cache_command.op.to_string(),
        cache_command.child_name.unwrap()
    );
    stream.write_all(data_2_server.as_bytes()).unwrap();
    stream.flush().unwrap();
}

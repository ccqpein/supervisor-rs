use std::env;
use std::error::Error;
use std::io::prelude::*;
use std::net::TcpStream;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use supervisor_rs::client::Command;

fn main() {
    let arguments = env::args();
    let change_2_vec = arguments.collect::<Vec<String>>();
    let cache_command = match Command::new_from_string(change_2_vec[1..].to_vec()) {
        Ok(c) => c,
        Err(e) => {
            println!("error: {}", e.description());
            return;
        }
    };

    let mut stream = if let Some(_) = cache_command.prep {
        //parse ip address
        //only accept ip address
        let addr = if let Some(des) = cache_command.obj {
            match des.parse::<IpAddr>() {
                Ok(ad) => ad,
                Err(e) => {
                    println!(
                        "something wrong when parse des ip address: {}, use 127.0.0.1 instead",
                        e
                    );
                    "127.0.0.1".parse::<IpAddr>().unwrap()
                }
            }
        } else {
            println!("there is no destination, send to local");
            //if no obj, give local address
            "127.0.0.1".parse::<IpAddr>().unwrap()
        };

        //creat socket
        let sock = SocketAddr::new(addr, 33889);
        match TcpStream::connect_timeout(&sock, Duration::new(5, 0)) {
            Ok(s) => s,
            Err(e) => {
                println!("error: {}", e.description());
                return;
            }
        }
    } else {
        //if don't have prep, give local address
        let sock = SocketAddr::new("127.0.0.1".parse::<IpAddr>().unwrap(), 33889);
        match TcpStream::connect_timeout(&sock, Duration::new(5, 0)) {
            Ok(s) => s,
            Err(e) => {
                println!("error: {}", e.description());
                return;
            }
        }
    };

    let data_2_server = format!(
        "{} {}",
        cache_command.op.to_string(),
        cache_command.child_name.unwrap_or(String::new())
    );

    //println!("{:?}", data_2_server);
    if let Err(e) = stream.write_all(data_2_server.as_bytes()) {
        println!("error: {}", e.description());
        return;
    };

    if let Err(e) = stream.flush() {
        println!("error: {}", e.description());
        return;
    };

    let mut response = String::new();
    if let Err(e) = stream.read_to_string(&mut response) {
        println!("error: {}", e.description());
        return;
    };
    print!("{}", response);
}

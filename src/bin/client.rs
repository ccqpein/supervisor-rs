use std::env;
use std::error::Error;
use std::io::prelude::*;
use std::io::Result;
use std::net::TcpStream;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use supervisor_rs::client::Command;

//:= TODO: get response from server
fn main() -> Result<()> {
    let arguments = env::args();
    let change_2_vec = arguments.collect::<Vec<String>>();
    let cache_command = Command::new_from_string(change_2_vec[1..].to_vec())?;

    //println!("{:?}", cache_command);

    let mut stream = if let Some(_) = cache_command.prep {
        //parse ip address
        let addr = if let Some(des) = cache_command.obj {
            match des.parse::<IpAddr>() {
                Ok(ad) => ad,
                Err(e) => {
                    println!("something wrong when parse des ip address: {}", e);
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
                println!("{}", e.description());
                return Err(e);
            }
        }
    } else {
        //if don't have prep, give local address
        let sock = SocketAddr::new("127.0.0.1".parse::<IpAddr>().unwrap(), 33889);
        match TcpStream::connect_timeout(&sock, Duration::new(5, 0)) {
            Ok(s) => s,
            Err(e) => {
                println!("{}", e.description());
                return Err(e);
            }
        }
    };

    let data_2_server = format!(
        "{} {}",
        cache_command.op.to_string(),
        cache_command.child_name.unwrap()
    );

    //println!("{:?}", data_2_server);
    stream.write_all(data_2_server.as_bytes())?;
    stream.flush()
}

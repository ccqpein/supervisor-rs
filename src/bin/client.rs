use std::env;
use std::io::prelude::*;
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::time::Duration;
use supervisor_rs::client::{Command, Ops};

const CANNOT_REACH_SERVER_ERROR: &'static str =
    "\nLooks like client cannot reach server side, make sure you start supervisor-rs-server on host you want to reach. \
Maybe it is network problem, or even worse, server app terminated. \
If server app terminated, all children were running become zombies. Check them out.";

fn main() {
    let arguments = env::args();
    let change_2_vec = arguments.collect::<Vec<String>>();
    let cache_command = match Command::new_from_string(change_2_vec[1..].to_vec()) {
        Ok(c) => c,
        Err(e) => {
            println!("error: {}", e);
            return;
        }
    };

    //println!("this is command {:?}", cache_command);

    if let Ops::Help = cache_command.get_ops() {
        println!("{}", help());
        return;
    }

    // build streams, parse all host
    let mut streams: Vec<TcpStream> = {
        if let Some(pairs) = cache_command.prep_obj_pairs() {
            //parse ip address
            //only accept ip address
            let ip_pair = pairs.iter().filter(|x| x.0.is_on());
            let addrs: Vec<IpAddr> = {
                // ip address format can be "127.0.0.1" or "127.0.0.1, 127.0.0.2"
                // this part need collect all addressed and *flatten* it
                let addresses = ip_pair
                    .map(|des| {
                        des.1
                            .split(|x| x == ',' || x == ' ')
                            .filter(|x| *x != "")
                            .collect::<Vec<&str>>()
                    })
                    .flatten()
                    .collect::<Vec<&str>>();

                // change ip to UpAddr
                let mut result: Vec<IpAddr> = vec![];
                for a in addresses {
                    match a.parse::<IpAddr>() {
                        Ok(ad) => result.push(ad),
                        Err(e) => {
                            println!("something wrong when parse des ip address {}: {}", a, e);
                            return;
                        }
                    };
                }
                result
            };

            //dbg!(&addrs);
            //creat socket
            let mut _streams: Vec<TcpStream> = vec![];
            for addr in addrs {
                let sock = SocketAddr::new(addr, 33889);
                match TcpStream::connect_timeout(&sock, Duration::new(5, 0)) {
                    Ok(s) => _streams.push(s),
                    Err(e) => {
                        println!("error of {}: {}; {}", addr, e, CANNOT_REACH_SERVER_ERROR);
                        return;
                    }
                };
            }
            _streams
        } else {
            vec![]
        }
    };

    if streams.len() == 0 {
        //if don't have prep, give local address
        let mut _streams: Vec<TcpStream> = vec![];
        let sock = SocketAddr::new("127.0.0.1".parse::<IpAddr>().unwrap(), 33889);
        match TcpStream::connect_timeout(&sock, Duration::new(5, 0)) {
            Ok(s) => _streams.push(s),
            Err(e) => {
                println!("error of 127.0.0.1: {}; {}", e, CANNOT_REACH_SERVER_ERROR);
                println!("maybe you are listening on ipv6 ::1?");
                return;
            }
        }
        streams = _streams
    }

    // Here to check/make encrypt data
    let data_2_server = if let Ok(d) = cache_command.generate_encrypt_wapper() {
        d.encrypt_to_bytes().unwrap()
    } else {
        cache_command.as_bytes()
    };

    //send same commands to all servers
    for mut stream in streams {
        let address = if let Ok(ad) = stream.peer_addr() {
            ad.to_string()
        } else {
            String::from("Unknow address")
        };

        if let Err(e) = stream.write_all(&data_2_server) {
            println!("Error from {}:\n {}", address, e);
            return;
        };

        if let Err(e) = stream.flush() {
            println!("Error from {}:\n {}", address, e);
            return;
        };

        let mut response = String::new();
        if let Err(e) = stream.read_to_string(&mut response) {
            println!("Error from {}:\n {}", address, e);
            return;
        };

        print!("Server {} response:\n{}", address, response);
    }
}

fn help() -> String {
    String::from(
        "\
Supervisor-rs used to manage precessings on server

supervisor-rs-server running on server side.
supervisor-rs-client used to send command to server side.

Example:

supervisor-rs-client start child1

supervisor-rs-client restart child1 on 192.168.1.1

Commands:

start/restart/kill/check/stop/kill

more detail:
https://github.com/ccqpein/supervisor-rs#usage
",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use supervisor_rs::client::*;

    #[test]
    fn ip_address_parse() {
        let mut cache_command = Command::new(Ops::Restart);
        cache_command.child_name = Some("child".to_string());
        cache_command.prep = Some(vec![Prepositions::On, Prepositions::On]);
        cache_command.obj = Some(vec![
            "192.168.1.1, 192.168.1.2".to_string(),
            "192.168.1.3".to_string(),
        ]);

        let pairs = cache_command.prep_obj_pairs().unwrap();
        let ip_pair = pairs.iter().filter(|x| x.0.is_on());
        let addrs: Vec<&str> = {
            let addresses = ip_pair
                .map(|des| {
                    des.1
                        .split(|x| x == ',' || x == ' ')
                        .filter(|x| *x != "")
                        .collect::<Vec<&str>>()
                })
                .flatten()
                .collect::<Vec<&str>>();
            addresses
        };

        assert_eq!(addrs, ["192.168.1.1", "192.168.1.2", "192.168.1.3"])
    }
}

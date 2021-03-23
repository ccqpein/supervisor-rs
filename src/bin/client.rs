use std::env;
use std::net::IpAddr;
use std::str::FromStr;
use supervisor_rs::client::*;

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

    if let Ops::Help = cache_command.get_ops() {
        println!("{}", help());
        return;
    }

    // build streams, parse all host
    let mut streams: Vec<ConnectionStream> = {
        if let Some(pairs) = cache_command.prep_obj_pairs() {
            // parse ip address
            // only accept ip address
            let ip_pair = pairs.iter().filter(|x| x.0.is_on());
            // ip address format can be "127.0.0.1" or "127.0.0.1, 127.0.0.2"
            // or "ssh://username@ipaddress"
            // or "ssh://username@ipaddress ,ssh://username1@ipaddress1"
            match ip_fields_parser(ip_pair) {
                Ok(addrs) => {
                    let mut a = vec![];
                    //creat socket
                    for addr in addrs {
                        match ConnectionStream::new(addr) {
                            Ok(s) => a.push(s),
                            Err(e) => {
                                println!("{}", e.to_string());
                                return;
                            }
                        }
                    }
                    a
                }
                Err(e) => {
                    println!("{}", e.to_string());
                    return;
                }
            }
        } else {
            vec![]
        }
    };

    if streams.len() == 0 {
        // If don't have prep, give local address (ipv4)
        streams = vec![match ConnectionStream::new(IpFields::Normal(
            IpAddr::from_str("127.0.0.1").unwrap(),
        )) {
            Ok(s) => s,
            Err(e) => {
                println!("{}", e.to_string());
                return;
            }
        }];
    }

    // Here to check/make encrypt data
    let data_2_server = if let Ok(d) = cache_command.generate_encrypt_wapper() {
        d.encrypt_to_bytes().unwrap()
    } else {
        cache_command.as_bytes()
    };

    //send same commands to all servers
    for mut stream in streams {
        print!(
            "Server {} response:\n{}",
            stream.address().unwrap(),
            stream.send_comm(&data_2_server).unwrap()
        );
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

use std::env;
use std::fs;
use std::net::IpAddr;
use std::path::Path;
use std::str::FromStr;
use supervisor_rs::arg::{normalize_client_args, ClientArgs, ClientSubcommand};
use supervisor_rs::client::*;
use supervisor_rs::noise;

fn main() {
    let normalized = normalize_client_args(env::args());
    let client_args = match ClientArgs::try_parse_from(normalized) {
        Ok(args) => args,
        Err(e) => {
            e.exit();
        }
    };

    if let ClientSubcommand::Keygen { ref out } = client_args.command {
        match noise::generate_keypair() {
            Ok(kp) => {
                let priv_hex = noise::to_hex(&kp.private);
                let pub_hex = noise::to_hex(&kp.public);
                if let Some(path_prefix) = out {
                    let key_path = format!("{}.key", path_prefix);
                    let pub_path = format!("{}.pub", path_prefix);
                    if let Err(e) = fs::write(&key_path, &priv_hex) {
                        eprintln!("Failed to write private key to {}: {}", key_path, e);
                        return;
                    }
                    if let Err(e) = fs::write(&pub_path, &pub_hex) {
                        eprintln!("Failed to write public key to {}: {}", pub_path, e);
                        return;
                    }
                    println!("Generated Noise (Curve25519) Keypair:");
                    println!("Wrote private key to: {}", key_path);
                    println!("Wrote public key to:  {}", pub_path);
                } else {
                    println!("Generated Noise (Curve25519) Keypair:");
                    println!("Private Key (hex): {}", priv_hex);
                    println!("Public Key (hex):  {}", pub_hex);
                }
            }
            Err(e) => eprintln!("Failed to generate keypair: {}", e),
        }
        return;
    }

    let cache_command = match client_args.to_command() {
        Ok(c) => c,
        Err(e) => {
            println!("error: {}", e);
            return;
        }
    };

    // Load Noise keys if Noise protocol is enabled
    let (noise_key, authorized_servers) = if client_args.noise || client_args.noise_key.is_some() {
        let key_path = client_args.noise_key.as_deref().unwrap_or_else(|| {
            if Path::new("./client.key").exists() {
                "./client.key"
            } else {
                "/tmp/client.key"
            }
        });

        let key = match noise::load_key_from_file(key_path) {
            Ok(k) => k,
            Err(e) => {
                eprintln!(
                    "Noise encryption enabled, but failed to load client private key from '{}': {}",
                    key_path, e
                );
                return;
            }
        };

        let mut auth_servers = Vec::new();
        if let Some(ref auth_path) = client_args.noise_authorized_keys {
            match noise::load_keys_from_path(auth_path) {
                Ok(keys) => auth_servers = keys,
                Err(e) => {
                    eprintln!(
                        "Warning: failed to load authorized server keys from '{}': {}",
                        auth_path, e
                    );
                }
            }
        } else if Path::new("./authorized_keys").exists() {
            if let Ok(keys) = noise::load_keys_from_path("./authorized_keys") {
                auth_servers = keys;
            }
        }

        (Some(key), Some(auth_servers))
    } else {
        (None, None)
    };

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
                        match ConnectionStream::new_with_noise(
                            addr,
                            noise_key.as_deref(),
                            authorized_servers.as_deref(),
                        ) {
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
        streams = vec![match ConnectionStream::new_with_noise(
            IpFields::Normal(IpAddr::from_str("127.0.0.1").unwrap()),
            noise_key.as_deref(),
            authorized_servers.as_deref(),
        ) {
            Ok(s) => s,
            Err(e) => {
                println!("{}", e.to_string());
                return;
            }
        }];
    }

    let data_2_server = cache_command.as_bytes();

    //send same commands to all servers
    for mut stream in streams {
        let resp = match stream.send_comm(&data_2_server) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error sending command: {}", e);
                continue;
            }
        };
        print!(
            "Server {} response:\n{}",
            stream.address().unwrap_or_default(),
            resp
        );
    }
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

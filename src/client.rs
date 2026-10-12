use anyhow::{anyhow, bail, Result};
use std::io::prelude::*;
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::str;
use std::time::Duration;

pub const CANNOT_REACH_SERVER_ERROR: &'static str =
    "Looks like client cannot reach server side, make sure you start supervisor-rs-server on host you want to reach. \
Maybe it is network problem, or even worse, server app terminated. \
If server app terminated, all children were running become zombies. Check them out.";

/// Client operations
///
/// All operations:
///
/// + Restart,
/// + Stop,
/// + Start,
/// + TryStart,
/// + Help,
/// + Kill,
/// + Check,
#[derive(Debug, PartialEq, Clone)]
pub enum Ops {
    Restart,
    Stop,
    Start,
    TryStart,

    Help,
    Info,

    Kill,
    Check,
}

impl Ops {
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "Restart" | "restart" => return Ok(Ops::Restart),
            "Start" | "start" => return Ok(Ops::Start),
            "Stop" | "stop" => return Ok(Ops::Stop),
            "Check" | "check" => return Ok(Ops::Check),
            "Kill" | "kill" => return Ok(Ops::Kill),
            "TryStart" | "Trystart" | "trystart" => return Ok(Ops::TryStart),
            "Info" | "INFO" | "InFo" | "info" => Ok(Ops::Info),
            "Help" | "help" | "-h" => return Ok(Ops::Help),
            _ => {
                bail!("no legal operations input");
            }
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Ops::Restart => return "restart".to_string(),
            Ops::Start => return "start".to_string(),
            Ops::Stop => return "stop".to_string(),
            Ops::Check => return "check".to_string(),
            Ops::Kill => return "kill".to_string(),
            Ops::TryStart => return "trystart".to_string(),
            Ops::Help => return "help".to_string(),
            Ops::Info => return "info".to_string(),
        }
    }

    pub fn is_op(s: &str) -> bool {
        if let Ok(_) = Self::from_str(s) {
            return true;
        }
        false
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Prepositions {
    On,
}

impl Prepositions {
    #[allow(dead_code)]
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "On" | "on" => Ok(Prepositions::On),
            "" => bail!("you miss prepositions"),
            _ => bail!("does not support {}", s),
        }
    }

    #[allow(dead_code)]
    fn is_prep(s: &str) -> bool {
        Self::from_str(s).is_ok()
    }

    pub fn is_on(&self) -> bool {
        *self == Self::On
    }
}

/// Command struct of client using to talk to server side
#[derive(Debug, PartialEq, Clone)]
pub struct Command {
    pub op: Ops,
    pub child_name: Option<String>,
    pub prep: Option<Vec<Prepositions>>,
    pub obj: Option<Vec<String>>,
    pub all: bool,
}

impl Command {
    pub fn new(op: Ops) -> Self {
        Command {
            op,
            child_name: None,
            prep: None,
            obj: None,
            all: false,
        }
    }

    pub fn new_from_string(s: Vec<String>) -> Result<Self> {
        Self::new_from_str(s.iter().map(|x| x.as_str()).collect())
    }

    /// Parse command using Clap with 0.x backwards compatibility
    pub fn new_from_str(s: Vec<&str>) -> Result<Self> {
        if s.is_empty() {
            bail!("no legal operations input");
        }

        if s[0] == "help" || s[0] == "Help" || s[0] == "-h" || s[0] == "--help" {
            return Ok(Self::new(Ops::Help));
        }

        let normalized = crate::legacy_arg::normalize_client_args(s);
        let client_args = crate::arg::ClientArgs::try_parse_from(normalized)
            .map_err(|e| anyhow!("{}", e))?;

        client_args.to_command()
    }

    pub fn get_ops(&self) -> Ops {
        self.op.clone()
    }

    pub fn prep_obj_pairs(&self) -> Option<Vec<(&Prepositions, &String)>> {
        if self.prep.is_none()
            || self.prep.as_ref().unwrap().len() != self.obj.as_ref().unwrap().len()
        {
            return None;
        }

        Some(
            self.prep
                .as_ref()
                .unwrap()
                .iter()
                .zip(self.obj.as_ref().unwrap().iter())
                .collect(),
        )
    }


    /// ops + ' ' + childname (or --all)
    /// and there are no Prepositions and Objects inside
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut cache = self.op.to_string().as_bytes().to_vec();
        if self.all {
            cache.push(b' ');
            cache.append(&mut b"--all".to_vec());
        } else if self.child_name.is_some() {
            cache.push(b' ');
            cache.append(
                &mut self
                    .child_name
                    .as_ref()
                    .unwrap_or(&String::new())
                    .as_bytes()
                    .to_vec(),
            );
        }

        cache.clone()
    }
}

#[derive(Debug, PartialEq)]
pub enum IpFields<'a> {
    Normal(IpAddr),
    SshIp { username: &'a str, ipaddr: IpAddr },
}

/// ip address parser, support normal ip address and ssh protocol
pub fn ip_fields_parser<'a>(
    ip_pair: impl Iterator<Item = &'a (&'a Prepositions, &'a String)>,
) -> Result<Vec<IpFields<'a>>> {
    let cache = ip_pair
        .map(|(_, ip)| ip.split(|x| x == ',' || x == ' ').filter(|x| !x.is_empty()))
        .flatten();

    let mut result = vec![];
    for s in cache {
        if s.starts_with("ssh://") {
            result.push(ssh_address_parse(s)?);
        } else {
            result.push(IpFields::Normal(s.parse::<IpAddr>()?));
        }
    }
    Ok(result)
}

/// ssh address has to follow 'ssh://username@ipaddress'
fn ssh_address_parse(address: &str) -> Result<IpFields<'_>> {
    let mut ll = address
        .split(|x| x == '/' || x == ':' || x == '@')
        .filter(|s| !s.is_empty());

    if ll.nth(0).unwrap_or("") != "ssh" {
        bail!("Only support ssh protocol");
    }

    Ok(IpFields::SshIp {
        username: ll.nth(0).ok_or_else(|| anyhow!("Username parse wrong"))?,
        ipaddr: ll
            .nth(0)
            .ok_or_else(|| anyhow!("IP address wrong"))?
            .parse::<IpAddr>()?,
    })
}

pub enum ConnectionStream {
    Tcp(TcpStream),
    Noise(crate::noise::NoiseSession, String),
    Ssh(crate::tunnel::TunnelStream, String),
}

impl ConnectionStream {
    /// generate new connections by using IpFields
    pub fn new(ip: IpFields<'_>) -> Result<ConnectionStream> {
        Self::new_with_noise(ip, None, None)
    }

    /// generate new connections with optional Noise encryption
    pub fn new_with_noise(
        ip: IpFields<'_>,
        noise_key: Option<&[u8]>,
        authorized_servers: Option<&[Vec<u8>]>,
    ) -> Result<ConnectionStream> {
        match ip {
            IpFields::Normal(addr) => {
                let sock = SocketAddr::new(addr, 33889);
                let tcp = TcpStream::connect_timeout(&sock, Duration::from_secs(5))
                    .map_err(|_| anyhow!(CANNOT_REACH_SERVER_ERROR))?;
                let _ = tcp.set_read_timeout(Some(Duration::from_secs(5)));
                let _ = tcp.set_write_timeout(Some(Duration::from_secs(5)));
                if let Some(priv_key) = noise_key {
                    let auth_srv = authorized_servers.unwrap_or(&[]);
                    let session = crate::noise::client_handshake(tcp, priv_key, auth_srv)
                        .map_err(|e| anyhow!("Noise handshake error: {}", e))?;
                    Ok(Self::Noise(session, addr.to_string()))
                } else {
                    Ok(Self::Tcp(tcp))
                }
            }
            IpFields::SshIp { username, ipaddr } => {
                let tunnel = crate::tunnel::TunnelStream::connect(username, ipaddr, 33889)?;
                let addr_str = format!("ssh://{}@{}", username, ipaddr);
                if let Some(priv_key) = noise_key {
                    let auth_srv = authorized_servers.unwrap_or(&[]);
                    let session = crate::noise::client_handshake(tunnel, priv_key, auth_srv)
                        .map_err(|e| anyhow!("Noise handshake error over SSH tunnel: {}", e))?;
                    Ok(Self::Noise(session, addr_str))
                } else {
                    Ok(Self::Ssh(tunnel, addr_str))
                }
            }
        }
    }

    /// send command to server during the built streams
    pub fn send_comm(&mut self, comm: &[u8]) -> Result<String> {
        match self {
            ConnectionStream::Noise(session, _) => {
                session.send(comm)?;
                let resp_bytes = session.recv()?;
                Ok(String::from_utf8(resp_bytes)?)
            }
            ConnectionStream::Tcp(s) => {
                s.write_all(comm)?;

                s.flush()?;

                let mut response = String::new();
                s.read_to_string(&mut response)?;

                Ok(response)
            }
            ConnectionStream::Ssh(tunnel, _) => {
                tunnel.write_all(comm)?;

                tunnel.flush()?;

                let mut response = String::new();
                tunnel.read_to_string(&mut response)?;

                Ok(response)
            }
        }
    }

    pub fn address(&self) -> Result<String> {
        Ok(match self {
            ConnectionStream::Noise(_, addr) => addr.clone(),
            ConnectionStream::Tcp(s) => s.peer_addr()?.to_string(),
            ConnectionStream::Ssh(_, addr) => addr.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_child_name() {
        let case0 = vec!["restart", "restart"];
        let comm = Command::new_from_str(case0);
        assert!(comm.is_err());
        assert_eq!(
            comm.err().unwrap().to_string(),
            "child name cannot be command"
        );
    }

    #[test]
    fn check_parser() {
        let case0 = vec!["restart", "child", "on", "host", "on", "host1"];
        assert_eq!(
            Command {
                op: Ops::Restart,
                child_name: Some("child".to_string()),
                prep: Some(vec![Prepositions::On, Prepositions::On]),
                obj: Some(vec!["host".to_string(), "host1".to_string()]),
                all: false,
            },
            Command::new_from_str(case0).unwrap()
        );

        // test2
        let case1 = vec!["restart", "child", "on", "host1, host2"]; // second hosts format
        assert_eq!(
            Command {
                op: Ops::Restart,
                child_name: Some("child".to_string()),
                prep: Some(vec![Prepositions::On]),
                obj: Some(vec!["host1, host2".to_string()]),
                all: false,
            },
            Command::new_from_str(case1).unwrap()
        );
    }

    #[test]
    fn check_make_pairs() {
        let case0 = Command {
            op: Ops::Restart,
            child_name: Some("child".to_string()),
            prep: None,
            obj: None,
            all: false,
        };
        assert_eq!(case0.prep_obj_pairs(), None);
    }


    #[test]
    fn test_ip_fields_parser() {
        let test = vec![
            (Prepositions::On, "127.0.0.1".to_string()),
            (Prepositions::On, "127.0.0.2, 127.0.0.3".to_string()),
            (Prepositions::On, "ssh://hello@127.0.0.1".to_string()),
            (
                Prepositions::On,
                "ssh://hello@127.0.0.1, ssh://hello@127.0.0.2".to_string(),
            ),
            (
                Prepositions::On,
                "ssh:/wrong@127.0.0.1, ssh://alsowrong127.0.0.2, sssh://jjj@oidde".to_string(),
            ),
        ];

        let test0 = test.iter().map(|(a, b)| (a, b)).collect::<Vec<_>>();

        assert_eq!(
            ip_fields_parser(vec![test0[0]].iter()).unwrap(),
            vec![IpFields::Normal("127.0.0.1".parse().unwrap())],
        );

        assert_eq!(
            ip_fields_parser(vec![test0[1]].iter()).unwrap(),
            vec![
                IpFields::Normal("127.0.0.2".parse().unwrap()),
                IpFields::Normal("127.0.0.3".parse().unwrap())
            ],
        );

        assert_eq!(
            ip_fields_parser(vec![test0[2]].iter()).unwrap(),
            vec![IpFields::SshIp {
                username: "hello",
                ipaddr: "127.0.0.1".parse().unwrap()
            }],
        );
        assert_eq!(
            ip_fields_parser(vec![test0[3]].iter()).unwrap(),
            vec![
                IpFields::SshIp {
                    username: "hello",
                    ipaddr: "127.0.0.1".parse().unwrap()
                },
                IpFields::SshIp {
                    username: "hello",
                    ipaddr: "127.0.0.2".parse().unwrap()
                }
            ],
        );
        assert!(ip_fields_parser(vec![test0[4]].iter()).is_err());
    }

    #[test]
    fn test_connection_stream_normal_tcp_send_comm() {
        use std::net::TcpListener;
        use std::thread;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let handle = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            let n = socket.read(&mut buf).unwrap();
            assert_eq!(&buf[..n], b"check");
            socket.write_all(b"All children good").unwrap();
        });

        let tcp = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let mut conn = ConnectionStream::Tcp(tcp);
        let resp = conn.send_comm(b"check").unwrap();
        assert_eq!(resp, "All children good");

        handle.join().unwrap();
    }
}

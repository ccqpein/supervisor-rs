use super::keys_handler::DataWrapper;
use ssh2::Session;
use std::io::prelude::*;
use std::io::{Error, ErrorKind, Result};
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
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "no legal operations input",
                ));
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

#[derive(Debug, PartialEq)]
pub enum Prepositions {
    On,
    With,
}

impl Prepositions {
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "On" | "on" => return Ok(Prepositions::On),
            "With" | "with" => return Ok(Prepositions::With),
            "" => {
                return Err(Error::new(ErrorKind::InvalidInput, "you miss prepositions"));
            }
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("does not support {}", s),
                ));
            }
        }
    }

    fn is_prep(s: &str) -> bool {
        if let Err(_) = Self::from_str(s) {
            return false;
        }
        true
    }

    pub fn is_on(&self) -> bool {
        if *self == Self::On {
            true
        } else {
            false
        }
    }

    fn is_with(&self) -> bool {
        if *self == Self::With {
            true
        } else {
            false
        }
    }
}

/// Command struct of client using to talk to server side
#[derive(Debug, PartialEq)]
pub struct Command {
    op: Ops,
    pub child_name: Option<String>,
    pub prep: Option<Vec<Prepositions>>,
    pub obj: Option<Vec<String>>,
}

impl Command {
    pub fn new(op: Ops) -> Self {
        Command {
            op,
            child_name: None,
            prep: None,
            obj: None,
        }
    }

    pub fn new_from_string(s: Vec<String>) -> Result<Self> {
        Self::new_from_str(s.iter().map(|x| x.as_str()).collect())
    }

    /// major parse function of command
    pub fn new_from_str(mut s: Vec<&str>) -> Result<Self> {
        // get op
        let mut re = Self::new(Ops::from_str(s[0])?);

        // kill and check do not have to have child name
        if re.op == Ops::Kill || re.op == Ops::Check || re.op == Ops::Info {
            s.drain(..1); // delete ops
            if s.len() >= 1 && !Prepositions::is_prep(s[0]) {
                // has child name
                if Ops::is_op(s[0]) {
                    // check child name
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        format!("child name cannot be command"),
                    ));
                }

                re.child_name = Some(s[0].to_string());
                s.drain(..1); // delete child name
            }
        } else {
            // other commands
            s.drain(..1); // delete ops
            if Ops::is_op(s[0]) {
                // check child name
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("child name cannot be command"),
                ));
            }

            re.child_name = Some(s[0].to_string());
            s.drain(..1); // delete child name
        }

        // parse all else
        if s.len() % 2 != 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "prep & obj arguments number should be even",
            ));
        }

        let mut i = 0;
        let mut prep_cache = vec![];
        let mut obj_cache = vec![];
        while i < s.len() {
            prep_cache.push(Prepositions::from_str(s[i])?);
            obj_cache.push(s[i + 1].to_string());
            i += 2;
        }

        if i != 0 {
            re.prep = Some(prep_cache);
            re.obj = Some(obj_cache);
        }

        Ok(re)
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

    pub fn generate_encrypt_wapper(&self) -> Result<DataWrapper> {
        if self.prep.is_none() {
            return Err(Error::new(
                ErrorKind::NotFound,
                "no key argument flag input",
            ));
        }

        if let Some(p) = self.prep.as_ref().unwrap().iter().position(|s| s.is_with()) {
            let keypath = if let Some(objs) = &self.obj {
                if let Some(f) = objs.get(p) {
                    f
                } else {
                    return Err(Error::new(
                        ErrorKind::NotFound,
                        "no key name argument flag input",
                    ));
                }
            } else {
                return Err(Error::new(
                    ErrorKind::NotFound,
                    "no key name argument flag input",
                ));
            };

            DataWrapper::new(&keypath, str::from_utf8(&self.as_bytes()).unwrap())
        } else {
            return Err(Error::new(
                ErrorKind::NotFound,
                "no key argument flag input",
            ));
        }
    }

    /// ops + ' ' + childname
    /// and there are no Prepositions and Objects inside
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut cache = self.op.to_string().as_bytes().to_vec();
        if self.child_name.is_some() {
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
) -> std::result::Result<Vec<IpFields<'a>>, String> {
    let cache = ip_pair
        .map(|(_, ip)| ip.split(|x| x == ',' || x == ' ').filter(|x| *x != ""))
        .flatten();

    let mut result = vec![];
    for s in cache {
        if s.starts_with("ssh://") {
            result.push(ssh_address_parse(s)?);
        } else {
            result.push(IpFields::Normal(
                s.parse::<IpAddr>().map_err(|e| e.to_string())?,
            ))
        }
    }
    Ok(result)
}

/// ssh address has to follow 'ssh://username@ipaddress'
fn ssh_address_parse(address: &str) -> std::result::Result<IpFields<'_>, String> {
    let mut ll = address
        .split(|x| x == '/' || x == ':' || x == '@')
        .filter(|s| *s != "");

    Ok(IpFields::SshIp {
        username: ll.nth(1).ok_or("Username parse wrong".to_string())?,
        ipaddr: ll
            .nth(0)
            .ok_or("IP address wrong".to_string())?
            .parse::<IpAddr>()
            .map_err(|e| e.to_string())?,
    })
}

pub enum ConnectionStream {
    Tcp(TcpStream),
    Ssh(Session, String),
}

impl ConnectionStream {
    /// generate new connections by using IpFields
    pub fn new(ip: IpFields<'_>) -> std::result::Result<ConnectionStream, String> {
        match ip {
            IpFields::Normal(addr) => {
                let sock = SocketAddr::new(addr, 33889);
                Ok(Self::Tcp(
                    TcpStream::connect_timeout(&sock, Duration::new(5, 0))
                        .map_err(|_| CANNOT_REACH_SERVER_ERROR)?,
                ))
            }
            IpFields::SshIp { username, ipaddr } => {
                let sock = SocketAddr::new(ipaddr, 22);
                let tcp = TcpStream::connect_timeout(&sock, Duration::new(5, 0))
                    .map_err(|_| CANNOT_REACH_SERVER_ERROR)?;
                let mut sess = Session::new().unwrap();
                sess.set_tcp_stream(tcp);
                sess.handshake().map_err(|e| e.to_string())?;
                sess.userauth_agent(username).map_err(|e| e.to_string())?;
                Ok(Self::Ssh(sess, ipaddr.to_string()))
            }
        }
    }

    /// send command to server during the built streams
    pub fn send_comm(&mut self, comm: &[u8]) -> Result<String> {
        match self {
            ConnectionStream::Tcp(s) => {
                s.write_all(comm)?;

                s.flush()?;

                let mut response = String::new();
                s.read_to_string(&mut response)?;

                Ok(response)
            }
            ConnectionStream::Ssh(s, _) => {
                let mut channel = s.channel_session()?;
                let mut head = "supervisor-rs-client ".to_string();
                head.push_str(str::from_utf8(comm).unwrap());
                channel.exec(head.as_str())?;

                let mut response = String::new();
                channel.read_to_string(&mut response)?;

                channel.wait_close()?;
                Ok(response)
            }
        }
    }

    pub fn address(&self) -> std::result::Result<String, String> {
        Ok(match self {
            ConnectionStream::Tcp(s) => s.peer_addr().map_err(|e| e.to_string())?.to_string(),
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
        let case0 = vec![
            "restart", "child", "with", "key", "on", "host", "on", "host1",
        ];
        assert_eq!(
            Command {
                op: Ops::Restart,
                child_name: Some("child".to_string()),
                prep: Some(vec![Prepositions::With, Prepositions::On, Prepositions::On]),
                obj: Some(vec![
                    "key".to_string(),
                    "host".to_string(),
                    "host1".to_string()
                ]),
            },
            Command::new_from_str(case0).unwrap()
        );

        // test2
        let case1 = vec!["restart", "child", "with", "key", "on", "host1, host2"]; // second hosts format
        assert_eq!(
            Command {
                op: Ops::Restart,
                child_name: Some("child".to_string()),
                prep: Some(vec![Prepositions::With, Prepositions::On]),
                obj: Some(vec!["key".to_string(), "host1, host2".to_string(),]),
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
        };
        assert_eq!(case0.prep_obj_pairs(), None);
    }

    #[test]
    fn check_generate_encrypt_wapper() -> Result<()> {
        let case0 = vec![
            "start",
            "child",
            "with",
            "./test/public.pem",
            "on",
            "127.0.0.1",
        ];
        let com0 = Command::new_from_str(case0)?;
        let dw = com0.generate_encrypt_wapper()?;
        assert_eq!(
            dw,
            DataWrapper::new("./test/public.pem", "start child").unwrap()
        );
        println!("{:?}", dw);

        let case1 = vec!["check", "with", "./test/public.pem", "on", "127.0.0.1"];
        let com0 = Command::new_from_str(case1)?;
        let dw = com0.generate_encrypt_wapper()?;
        assert_eq!(dw, DataWrapper::new("./test/public.pem", "check").unwrap());
        Ok(())
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

        let test0 = test.iter().map(|(ref a, ref b)| (a, b)).collect::<Vec<_>>();

        assert_eq!(
            ip_fields_parser(vec![test0[0]].iter()),
            Ok(vec![IpFields::Normal("127.0.0.1".parse().unwrap())]),
        );

        assert_eq!(
            ip_fields_parser(vec![test0[1]].iter()),
            Ok(vec![
                IpFields::Normal("127.0.0.2".parse().unwrap()),
                IpFields::Normal("127.0.0.3".parse().unwrap())
            ]),
        );

        assert_eq!(
            ip_fields_parser(vec![test0[2]].iter()),
            Ok(vec![IpFields::SshIp {
                username: "hello",
                ipaddr: "127.0.0.1".parse().unwrap()
            }]),
        );
        assert_eq!(
            ip_fields_parser(vec![test0[3]].iter()),
            Ok(vec![
                IpFields::SshIp {
                    username: "hello",
                    ipaddr: "127.0.0.1".parse().unwrap()
                },
                IpFields::SshIp {
                    username: "hello",
                    ipaddr: "127.0.0.2".parse().unwrap()
                }
            ]),
        );
        assert!(ip_fields_parser(vec![test0[4]].iter()).is_err());
    }
}

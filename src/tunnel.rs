use ssh2::{Channel, Session};
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

pub const CANNOT_REACH_SERVER_ERROR: &str =
    "Looks like client cannot reach server side, make sure you start supervisor-rs-server on host you want to reach. Maybe it is network problem, or even worse, server app terminated. If server app terminated, all children were running become zombies. Check them out.";

/// A bidirectional stream through an SSH direct-tcpip channel.
pub struct TunnelStream {
    channel: Channel,
}

impl Read for TunnelStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.channel.read(buf)
    }
}

impl Write for TunnelStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.channel.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.channel.flush()
    }
}

impl TunnelStream {
    /// Connect to remote host over SSH and open a direct-tcpip channel to target_port
    pub fn connect(
        username: &str,
        ipaddr: std::net::IpAddr,
        target_port: u16,
    ) -> Result<Self, String> {
        let sock = SocketAddr::new(ipaddr, 22);
        let tcp = TcpStream::connect_timeout(&sock, Duration::new(5, 0))
            .map_err(|_| CANNOT_REACH_SERVER_ERROR)?;

        let mut sess = Session::new().map_err(|e| e.to_string())?;
        sess.set_tcp_stream(tcp);
        sess.handshake().map_err(|e| e.to_string())?;

        let mut agent = sess.agent().map_err(|e| e.to_string())?;
        agent.connect().map_err(|e| e.to_string())?;
        agent.list_identities().map_err(|e| e.to_string())?;

        let mut authed = false;
        for identity in agent.identities().map_err(|e| e.to_string())? {
            if agent.userauth(username, &identity).is_ok() {
                authed = true;
                break;
            }
        }

        if !authed {
            return Err("[Session(-18)] Username/PublicKey combination invalid".to_string());
        }

        let channel = sess
            .channel_direct_tcpip("127.0.0.1", target_port, None)
            .map_err(|e| format!("Failed to open direct-tcpip channel to port {}: {}", target_port, e))?;

        Ok(TunnelStream { channel })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;
    use std::str::FromStr;

    #[test]
    fn test_tunnel_connect_unreachable() {
        // Connecting to a non-existent SSH port should fail with cannot reach server error
        let ip = IpAddr::from_str("127.0.0.1").unwrap();
        let res = TunnelStream::connect("dummy_user", ip, 33889);
        assert!(res.is_err());
    }

    #[test]
    fn test_tunnel_stream_implements_noise_stream() {
        fn assert_stream<T: crate::noise::Stream>() {}
        assert_stream::<TunnelStream>();
    }
}

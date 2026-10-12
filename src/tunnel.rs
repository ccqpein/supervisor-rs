use anyhow::{anyhow, bail, Result};
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
    ) -> Result<Self> {
        let sock = SocketAddr::new(ipaddr, 22);
        let tcp = TcpStream::connect_timeout(&sock, Duration::from_secs(5))
            .map_err(|_| anyhow!(CANNOT_REACH_SERVER_ERROR))?;
        let _ = tcp.set_read_timeout(Some(Duration::from_secs(5)));
        let _ = tcp.set_write_timeout(Some(Duration::from_secs(5)));

        let mut sess = Session::new()?;
        sess.set_tcp_stream(tcp);
        sess.set_timeout(5000); // 5 seconds timeout for all SSH channel and session operations
        sess.handshake()?;

        let mut agent = sess.agent()?;
        agent.connect()?;
        agent.list_identities()?;

        let mut authed = false;
        for identity in agent.identities()? {
            if agent.userauth(username, &identity).is_ok() {
                authed = true;
                break;
            }
        }

        if !authed {
            bail!("[Session(-18)] Username/PublicKey combination invalid");
        }

        let channel = sess
            .channel_direct_tcpip("127.0.0.1", target_port, None)
            .map_err(|e| anyhow!("Failed to open direct-tcpip channel to port {}: {}", target_port, e))?;

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

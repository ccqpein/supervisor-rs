use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;

pub const NOISE_PATTERN: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";
pub const MAX_MESSAGE_LEN: usize = 65535;

/// Convert bytes to hex string
pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Convert hex string to bytes
pub fn from_hex(s: &str) -> io::Result<Vec<u8>> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid hex string length",
        ));
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
        })
        .collect()
}

/// Generate a new 32-byte Curve25519 keypair for Noise protocol
pub fn generate_keypair() -> Result<snow::Keypair, snow::Error> {
    let builder = snow::Builder::new(NOISE_PATTERN.parse().unwrap());
    builder.generate_keypair()
}

/// Derive a 32-byte Curve25519 public key from a 32-byte private key
pub fn derive_public_key(priv_key: &[u8]) -> io::Result<Vec<u8>> {
    use snow::resolvers::CryptoResolver;
    let mut dh = snow::resolvers::DefaultResolver
        .resolve_dh(&snow::params::DHChoice::Curve25519)
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "DH resolver not available"))?;
    if priv_key.len() != dh.priv_len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Invalid private key length: expected {}, got {}",
                dh.priv_len(),
                priv_key.len()
            ),
        ));
    }
    dh.set(priv_key);
    Ok(dh.pubkey().to_vec())
}

/// Parse a 32-byte key from file content (supports 64 hex characters or 32 raw bytes)
pub fn parse_key(content: &[u8]) -> io::Result<Vec<u8>> {
    let s = String::from_utf8_lossy(content).trim().to_string();
    let cleaned: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if cleaned.len() == 64 {
        if let Ok(bytes) = from_hex(&cleaned) {
            if bytes.len() == 32 {
                return Ok(bytes);
            }
        }
    }
    if content.len() == 32 {
        return Ok(content.to_vec());
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "Invalid 32-byte key (expected 64 hex characters or 32 raw bytes, got {} bytes)",
            content.len()
        ),
    ))
}

/// Load a 32-byte key from a specified file
pub fn load_key_from_file(path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    let mut file = File::open(path.as_ref())?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    parse_key(&buf)
}

/// Load keys from either a directory of key files or a single file
pub fn load_keys_from_path(path: impl AsRef<Path>) -> io::Result<Vec<Vec<u8>>> {
    let p = path.as_ref();
    if !p.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Path not found: {:?}", p),
        ));
    }

    let mut keys = Vec::new();
    if p.is_dir() {
        for entry in fs::read_dir(p)? {
            let entry = entry?;
            let file_path = entry.path();
            if file_path.is_file() {
                if let Ok(key) = load_key_from_file(&file_path) {
                    keys.push(key);
                }
            }
        }
    } else {
        // Read file line by line for multiple hex keys or single key
        let content = fs::read_to_string(p)?;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Ok(key) = parse_key(trimmed.as_bytes()) {
                keys.push(key);
            }
        }
        if keys.is_empty() {
            // Try raw binary fallback
            let raw = fs::read(p)?;
            if let Ok(key) = parse_key(&raw) {
                keys.push(key);
            }
        }
    }

    Ok(keys)
}

/// Trait representing any bidirectional byte stream suitable for Noise transport
pub trait Stream: io::Read + io::Write + Send {}
impl<T: io::Read + io::Write + Send> Stream for T {}

/// Send a 2-byte length-prefixed frame
pub fn send_frame<W: io::Write + ?Sized>(stream: &mut W, data: &[u8]) -> io::Result<()> {
    if data.len() > MAX_MESSAGE_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Noise frame exceeds maximum size of 65535 bytes",
        ));
    }
    let len_bytes = (data.len() as u16).to_be_bytes();
    stream.write_all(&len_bytes)?;
    stream.write_all(data)?;
    stream.flush()?;
    Ok(())
}

/// Read a 2-byte length-prefixed frame
pub fn recv_frame<R: io::Read + ?Sized>(stream: &mut R) -> io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 2];
    stream.read_exact(&mut len_buf)?;
    let len = u16::from_be_bytes(len_buf) as usize;
    if len > MAX_MESSAGE_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Noise frame exceeds maximum size of 65535 bytes",
        ));
    }
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf)?;
    Ok(buf)
}

/// Encrypted Noise transport session
pub struct NoiseSession {
    pub stream: Box<dyn Stream>,
    pub transport: snow::TransportState,
}

impl NoiseSession {
    /// Send encrypted plaintext message
    pub fn send(&mut self, plaintext: &[u8]) -> io::Result<()> {
        let mut cipher_buf = vec![0u8; plaintext.len() + 16]; // 16-byte Poly1305 MAC tag
        let len = self
            .transport
            .write_message(plaintext, &mut cipher_buf)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        send_frame(&mut self.stream, &cipher_buf[..len])
    }

    /// Receive and decrypt message
    pub fn recv(&mut self) -> io::Result<Vec<u8>> {
        let cipher_frame = recv_frame(&mut self.stream)?;
        let mut plain_buf = vec![0u8; cipher_frame.len()];
        let len = self
            .transport
            .read_message(&cipher_frame, &mut plain_buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        plain_buf.truncate(len);
        Ok(plain_buf)
    }

    /// Get authenticated static public key of the remote peer
    pub fn remote_static_key(&self) -> Option<&[u8]> {
        self.transport.get_remote_static()
    }
}

/// Perform server-side Noise_XX handshake and authenticate client against authorized keys
pub fn server_handshake<S: Stream + 'static>(
    mut stream: S,
    server_priv_key: &[u8],
    authorized_clients: &[Vec<u8>],
) -> io::Result<NoiseSession> {
    let mut responder = snow::Builder::new(NOISE_PATTERN.parse().unwrap())
        .local_private_key(server_priv_key)
        .build_responder()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    let mut buf = [0u8; MAX_MESSAGE_LEN];

    // Message 1 (client -> server): e (Noise_XX ephemeral key, exactly 32 bytes)
    let mut len_buf1 = [0u8; 2];
    stream.read_exact(&mut len_buf1)?;
    let len1 = u16::from_be_bytes(len_buf1) as usize;
    if len1 != 32 {
        let _ = stream.write_all(b"Error: Server requires Noise protocol encryption. Client must connect with --noise for handshaking.\n");
        let _ = stream.flush();
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Invalid Noise Message 1: expected 32-byte frame, received length header {}",
                len1
            ),
        ));
    }
    let mut msg1 = vec![0u8; len1];
    stream.read_exact(&mut msg1)?;
    responder
        .read_message(&msg1, &mut buf)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    // Message 2 (server -> client): e, ee, s, es
    let len2 = responder
        .write_message(&[], &mut buf)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    send_frame(&mut stream, &buf[..len2])?;

    // Message 3 (client -> server): s, se
    let msg3 = recv_frame(&mut stream)?;
    responder
        .read_message(&msg3, &mut buf)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    // Verify client static public key
    let client_key = responder.get_remote_static().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            "No client static public key received",
        )
    })?;

    if !authorized_clients.is_empty()
        && !authorized_clients
            .iter()
            .any(|k| k.as_slice() == client_key)
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "Client public key [{}] is not authorized",
                to_hex(client_key)
            ),
        ));
    }

    let transport = responder
        .into_transport_mode()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    Ok(NoiseSession {
        stream: Box::new(stream),
        transport,
    })
}

/// Perform client-side Noise_XX handshake and authenticate server against authorized keys
pub fn client_handshake<S: Stream + 'static>(
    mut stream: S,
    client_priv_key: &[u8],
    authorized_servers: &[Vec<u8>],
) -> io::Result<NoiseSession> {
    let mut initiator = snow::Builder::new(NOISE_PATTERN.parse().unwrap())
        .local_private_key(client_priv_key)
        .build_initiator()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    let mut buf = [0u8; MAX_MESSAGE_LEN];

    // Message 1 (client -> server): e
    let len1 = initiator
        .write_message(&[], &mut buf)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    send_frame(&mut stream, &buf[..len1])?;

    // Message 2 (server -> client): e, ee, s, es
    let msg2 = recv_frame(&mut stream)?;
    initiator
        .read_message(&msg2, &mut buf)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    // Verify server static public key
    if let Some(server_key) = initiator.get_remote_static() {
        if !authorized_servers.is_empty()
            && !authorized_servers
                .iter()
                .any(|k| k.as_slice() == server_key)
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!(
                    "Server public key [{}] is not in client's authorized_keys",
                    to_hex(server_key)
                ),
            ));
        }
    }

    // Message 3 (client -> server): s, se
    let len3 = initiator
        .write_message(&[], &mut buf)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    send_frame(&mut stream, &buf[..len3])?;

    let transport = initiator
        .into_transport_mode()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    Ok(NoiseSession {
        stream: Box::new(stream),
        transport,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    #[test]
    fn test_hex_roundtrip() {
        let raw = vec![0x00, 0x01, 0x0f, 0x10, 0xff, 0xaa];
        let hex = to_hex(&raw);
        assert_eq!(hex, "00010f10ffaa");
        let decoded = from_hex(&hex).unwrap();
        assert_eq!(decoded, raw);
    }

    #[test]
    fn test_generate_and_parse_key() {
        let kp = generate_keypair().unwrap();
        assert_eq!(kp.public.len(), 32);
        assert_eq!(kp.private.len(), 32);

        let hex_pub = to_hex(&kp.public);
        let parsed = parse_key(hex_pub.as_bytes()).unwrap();
        assert_eq!(parsed, kp.public);

        let derived = derive_public_key(&kp.private).unwrap();
        assert_eq!(derived, kp.public);
    }

    #[test]
    fn test_mutual_auth_noise_handshake() {
        let server_kp = generate_keypair().unwrap();
        let client_kp = generate_keypair().unwrap();

        let authorized_clients = vec![client_kp.public.clone()];
        let authorized_servers = vec![server_kp.public.clone()];

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let s_priv = server_kp.private.clone();
        let server_handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut session = server_handshake(stream, &s_priv, &authorized_clients).unwrap();
            let cmd = session.recv().unwrap();
            assert_eq!(String::from_utf8(cmd).unwrap(), "start test_child");
            session.send(b"ok").unwrap();
        });

        let stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let mut client_session =
            client_handshake(stream, &client_kp.private, &authorized_servers).unwrap();

        client_session.send(b"start test_child").unwrap();
        let resp = client_session.recv().unwrap();
        assert_eq!(String::from_utf8(resp).unwrap(), "ok");

        server_handle.join().unwrap();
    }

    #[test]
    fn test_server_rejects_unauthorized_client() {
        let server_kp = generate_keypair().unwrap();
        let authorized_client = generate_keypair().unwrap();
        let rogue_client = generate_keypair().unwrap();

        let authorized_clients = vec![authorized_client.public.clone()];

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let s_priv = server_kp.private.clone();
        let server_handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let res = server_handshake(stream, &s_priv, &authorized_clients);
            assert!(res.is_err());
            assert_eq!(res.err().unwrap().kind(), io::ErrorKind::PermissionDenied);
        });

        let stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        // Client without server verification for this test
        let res = client_handshake(stream, &rogue_client.private, &[]);
        // Either server closes stream during msg3 or client gets error
        let _ = res;

        server_handle.join().unwrap();
    }

    #[test]
    fn test_client_rejects_unauthorized_server() {
        let server_kp = generate_keypair().unwrap();
        let another_server_kp = generate_keypair().unwrap();
        let client_kp = generate_keypair().unwrap();

        // Client only authorizes `another_server_kp`
        let authorized_servers = vec![another_server_kp.public.clone()];

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let s_priv = server_kp.private.clone();
        let server_handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let _ = server_handshake(stream, &s_priv, &[]);
        });

        let stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let res = client_handshake(stream, &client_kp.private, &authorized_servers);
        assert!(res.is_err());
        assert_eq!(res.err().unwrap().kind(), io::ErrorKind::PermissionDenied);

        server_handle.join().unwrap();
    }

    #[test]
    fn test_load_keys_from_dir_and_file() {
        let temp_dir = std::env::temp_dir().join(format!("noise_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        let kp1 = generate_keypair().unwrap();
        let kp2 = generate_keypair().unwrap();

        let file1 = temp_dir.join("alice.pub");
        let file2 = temp_dir.join("bob.pub");
        fs::write(&file1, to_hex(&kp1.public)).unwrap();
        fs::write(&file2, to_hex(&kp2.public)).unwrap();

        // Load single key
        let loaded1 = load_key_from_file(&file1).unwrap();
        assert_eq!(loaded1, kp1.public);

        // Load from directory
        let dir_keys = load_keys_from_path(&temp_dir).unwrap();
        assert_eq!(dir_keys.len(), 2);
        assert!(dir_keys.contains(&kp1.public));
        assert!(dir_keys.contains(&kp2.public));

        // Load from multi-line file
        let list_file = temp_dir.join("authorized_keys");
        let multi_content = format!(
            "# Comment line\n{}\n{}\n",
            to_hex(&kp1.public),
            to_hex(&kp2.public)
        );
        fs::write(&list_file, multi_content).unwrap();
        let file_keys = load_keys_from_path(&list_file).unwrap();
        assert_eq!(file_keys.len(), 2);
        assert!(file_keys.contains(&kp1.public));
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_server_rejects_unencrypted_client_immediately() {
        use std::io::Write;
        use std::time::Duration;

        let server_kp = generate_keypair().unwrap();
        let s_priv = server_kp.private.clone();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let server_handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let res = server_handshake(stream, &s_priv, &[]);
            assert!(res.is_err());
            assert_eq!(res.err().unwrap().kind(), io::ErrorKind::InvalidData);
        });

        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        stream.write_all(b"check").unwrap();
        stream.flush().unwrap();

        let mut response = String::new();
        let _ = stream.read_to_string(&mut response);
        // Returns immediately without hanging!
        server_handle.join().unwrap();
    }
}

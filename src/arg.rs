use clap::{Parser, Subcommand};
use std::env;
use std::io::{Error, ErrorKind, Result};

use crate::client::{Command, Ops, Prepositions};

/// Command line arguments for supervisor-rs-server
#[derive(Parser, Debug, Clone, PartialEq)]
#[command(
    name = "supervisor-rs-server",
    author = "ccQpein",
    version,
    about = "Supervisor-rs server daemon for managing child processes."
)]
pub struct ServerArgs {
    /// Path to server configuration YAML file [default: /tmp/server.yml]
    #[arg(value_name = "CONFIG")]
    pub config: Option<String>,

    /// Path to server configuration YAML file
    #[arg(short = 'c', long = "config", value_name = "CONFIG_PATH")]
    pub config_opt: Option<String>,

    /// Enable Noise protocol encryption
    #[arg(short = 'n', long = "noise")]
    pub noise: bool,

    /// Path to server Noise private key
    #[arg(long = "noise-key", value_name = "KEY_PATH")]
    pub noise_key: Option<String>,

    /// Path to authorized client public keys directory or file
    #[arg(long = "noise-authorized-keys", value_name = "KEYS_PATH")]
    pub noise_authorized_keys: Option<String>,

    /// Generate a new Noise Curve25519 keypair and exit
    #[arg(long = "keygen")]
    pub keygen: bool,
}

impl ServerArgs {
    pub fn try_parse_from<I, T>(itr: I) -> std::result::Result<Self, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        <Self as clap::Parser>::try_parse_from(itr)
    }

    /// Return configuration file path (empty string indicates default /tmp/server.yml)
    pub fn config_path(&self) -> &str {
        if let Some(ref c) = self.config_opt {
            c.as_str()
        } else if let Some(ref c) = self.config {
            c.as_str()
        } else {
            ""
        }
    }
}

/// Command line arguments for supervisor-rs-client
#[derive(Parser, Debug, Clone, PartialEq)]
#[command(
    name = "supervisor-rs-client",
    author = "ccQpein",
    version,
    about = "Supervisor-rs client used to send commands to server side.",
    after_help = "Examples:\n  supervisor-rs-client start child1\n  supervisor-rs-client restart child1 on 192.168.1.1\n  supervisor-rs-client restart child1 --on 192.168.1.1\n  supervisor-rs-client check on 192.168.1.1\n  supervisor-rs-client restart child1 -o 192.168.1.1 --noise -k client.key --noise-authorized-keys authorized_keys/\n  supervisor-rs-client keygen\n\nMore details:\n  https://github.com/ccqpein/supervisor-rs#usage"
)]
pub struct ClientArgs {
    #[command(subcommand)]
    pub command: ClientSubcommand,

    /// Remote host address(es) to send command to (e.g. 192.168.1.1, ssh://user@host)
    #[arg(short = 'o', long = "on", global = true, action = clap::ArgAction::Append, value_name = "HOST")]
    pub on: Vec<String>,

    /// Enable Noise protocol encryption
    #[arg(short = 'n', long = "noise", global = true)]
    pub noise: bool,

    /// Path to client Noise private key
    #[arg(short = 'k', long = "noise-key", global = true, value_name = "KEY_PATH")]
    pub noise_key: Option<String>,

    /// Path to authorized server public keys directory or file
    #[arg(long = "noise-authorized-keys", global = true, value_name = "KEYS_PATH")]
    pub noise_authorized_keys: Option<String>,
}

impl ClientArgs {
    pub fn try_parse_from<I, T>(itr: I) -> std::result::Result<Self, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        <Self as clap::Parser>::try_parse_from(itr)
    }

    /// Parse arguments from environment, supporting both Clap options and 0.x preposition syntax
    pub fn parse_from_env() -> std::result::Result<Self, clap::Error> {
        let raw_args: Vec<String> = env::args().collect();
        let normalized = normalize_client_args(raw_args);
        Self::try_parse_from(normalized)
    }

    /// Convert into legacy Command struct for compatibility with 0.x codebase
    pub fn to_command(&self) -> Result<Command> {
        if let ClientSubcommand::Keygen { .. } = self.command {
            return Err(Error::new(
                ErrorKind::Other,
                "Keygen is handled locally and cannot be converted to server command",
            ));
        }
        self.command.validate()?;

        let op = self.command.to_op();
        let child_name = self.command.child_name().map(|s| s.to_string());

        let mut prep = vec![];
        let mut obj = vec![];

        for host in &self.on {
            prep.push(Prepositions::On);
            obj.push(host.clone());
        }

        let (prep_opt, obj_opt) = if prep.is_empty() {
            (None, None)
        } else {
            (Some(prep), Some(obj))
        };

        Ok(Command {
            op,
            child_name,
            prep: prep_opt,
            obj: obj_opt,
        })
    }
}

/// Supported client subcommands
#[derive(Subcommand, Debug, Clone, PartialEq)]
pub enum ClientSubcommand {
    /// Restart child on server
    #[command(alias = "Restart")]
    Restart {
        /// Name of the child process
        child: String,
    },

    /// Start new child on server
    #[command(alias = "Start")]
    Start {
        /// Name of the child process
        child: String,
    },

    /// Stop running child on server
    #[command(alias = "Stop")]
    Stop {
        /// Name of the child process, or 'all' to stop all
        child: String,
    },

    /// Try to start child: restart if running, start if stopped
    #[command(alias = "Trystart", alias = "TryStart")]
    Trystart {
        /// Name of the child process
        child: String,
    },

    /// Check status of children
    #[command(alias = "Check")]
    Check {
        /// Name of the child process (optional)
        child: Option<String>,
    },

    /// Get general information of server
    #[command(alias = "Info", alias = "INFO", alias = "InFo")]
    Info {
        /// Child or section name (optional)
        child: Option<String>,
    },

    /// Terminate supervisor server daemon
    #[command(alias = "Kill")]
    Kill {
        /// Optional argument
        child: Option<String>,
    },

    /// Generate a new Noise Curve25519 keypair
    #[command(alias = "Keygen")]
    Keygen {
        /// Optional output path prefix (<out>.key and <out>.pub)
        out: Option<String>,
    },
}

impl ClientSubcommand {
    pub fn child_name(&self) -> Option<&str> {
        match self {
            ClientSubcommand::Restart { child } => Some(child),
            ClientSubcommand::Start { child } => Some(child),
            ClientSubcommand::Stop { child } => Some(child),
            ClientSubcommand::Trystart { child } => Some(child),
            ClientSubcommand::Check { child } => child.as_deref(),
            ClientSubcommand::Info { child } => child.as_deref(),
            ClientSubcommand::Kill { child } => child.as_deref(),
            ClientSubcommand::Keygen { .. } => None,
        }
    }

    pub fn to_op(&self) -> Ops {
        match self {
            ClientSubcommand::Restart { .. } => Ops::Restart,
            ClientSubcommand::Start { .. } => Ops::Start,
            ClientSubcommand::Stop { .. } => Ops::Stop,
            ClientSubcommand::Trystart { .. } => Ops::TryStart,
            ClientSubcommand::Check { .. } => Ops::Check,
            ClientSubcommand::Info { .. } => Ops::Info,
            ClientSubcommand::Kill { .. } => Ops::Kill,
            ClientSubcommand::Keygen { .. } => Ops::Help,
        }
    }

    pub fn validate(&self) -> Result<()> {
        if let Some(name) = self.child_name() {
            if Ops::is_op(name) {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "child name cannot be command",
                ));
            }
        }
        Ok(())
    }
}

/// Normalize arguments to support 0.x preposition syntax ("on <host>", "with <key>")
/// as well as standard clap flags ("--on <host>", "--with <key>").
pub fn normalize_client_args<I, T>(raw_args: I) -> Vec<String>
where
    I: IntoIterator<Item = T>,
    T: AsRef<str>,
{
    let args: Vec<String> = raw_args.into_iter().map(|s| s.as_ref().to_string()).collect();
    if args.is_empty() {
        return vec!["supervisor-rs-client".to_string()];
    }

    let has_bin = args[0].contains("supervisor-rs-client")
        || args[0].starts_with('/')
        || args[0].starts_with("./");

    let mut normalized = if has_bin {
        vec![]
    } else {
        vec!["supervisor-rs-client".to_string()]
    };

    let mut expecting_value = false;

    for (i, token) in args.into_iter().enumerate() {
        if i == 0 && has_bin {
            normalized.push(token);
            continue;
        }

        if expecting_value {
            normalized.push(token);
            expecting_value = false;
            continue;
        }

        if token == "-o" || token == "--on" {
            expecting_value = true;
            normalized.push(token);
        } else if token == "on" || token == "On" {
            normalized.push("--on".to_string());
            expecting_value = true;
        } else if token == "-k"
            || token == "--noise-key"
            || token == "--noise-authorized-keys"
            || token == "-c"
            || token == "--config"
        {
            expecting_value = true;
            normalized.push(token);
        } else {
            normalized.push(token);
        }
    }

    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_args_default() {
        let args = ServerArgs::try_parse_from(&["supervisor-rs-server"]).unwrap();
        assert_eq!(args.config_path(), "");
    }

    #[test]
    fn test_server_args_positional() {
        let args = ServerArgs::try_parse_from(&["supervisor-rs-server", "/path/to/server.yml"]).unwrap();
        assert_eq!(args.config_path(), "/path/to/server.yml");
    }

    #[test]
    fn test_server_args_flag() {
        let args = ServerArgs::try_parse_from(&["supervisor-rs-server", "-c", "/path/to/server.yml"]).unwrap();
        assert_eq!(args.config_path(), "/path/to/server.yml");

        let args = ServerArgs::try_parse_from(&["supervisor-rs-server", "--config", "/path/to/server.yml"]).unwrap();
        assert_eq!(args.config_path(), "/path/to/server.yml");
    }

    #[test]
    fn test_client_args_modern_flags() {
        let normalized = normalize_client_args(&[
            "supervisor-rs-client",
            "restart",
            "child0",
            "--on",
            "127.0.0.1",
        ]);
        let client_args = ClientArgs::try_parse_from(normalized).unwrap();
        assert_eq!(
            client_args.command,
            ClientSubcommand::Restart {
                child: "child0".to_string()
            }
        );
        assert_eq!(client_args.on, vec!["127.0.0.1"]);
    }

    #[test]
    fn test_client_args_short_flags() {
        let normalized = normalize_client_args(&[
            "supervisor-rs-client",
            "restart",
            "child0",
            "-o",
            "127.0.0.1",
        ]);
        let client_args = ClientArgs::try_parse_from(normalized).unwrap();
        assert_eq!(client_args.on, vec!["127.0.0.1"]);
    }

    #[test]
    fn test_client_args_0x_style() {
        let normalized = normalize_client_args(&[
            "supervisor-rs-client",
            "restart",
            "child0",
            "on",
            "127.0.0.1",
        ]);
        let client_args = ClientArgs::try_parse_from(normalized).unwrap();
        assert_eq!(
            client_args.command,
            ClientSubcommand::Restart {
                child: "child0".to_string()
            }
        );
        assert_eq!(client_args.on, vec!["127.0.0.1"]);

        let cmd = client_args.to_command().unwrap();
        assert_eq!(cmd.op, Ops::Restart);
        assert_eq!(cmd.child_name, Some("child0".to_string()));
        assert_eq!(
            cmd.prep,
            Some(vec![Prepositions::On])
        );
        assert_eq!(
            cmd.obj,
            Some(vec!["127.0.0.1".to_string()])
        );
    }

    #[test]
    fn test_client_args_check_optional() {
        let normalized = normalize_client_args(&["supervisor-rs-client", "check"]);
        let client_args = ClientArgs::try_parse_from(normalized).unwrap();
        assert_eq!(client_args.command, ClientSubcommand::Check { child: None });

        let normalized = normalize_client_args(&["supervisor-rs-client", "check", "child1"]);
        let client_args = ClientArgs::try_parse_from(normalized).unwrap();
        assert_eq!(
            client_args.command,
            ClientSubcommand::Check {
                child: Some("child1".to_string())
            }
        );
    }

    #[test]
    fn test_client_args_validate_child_name() {
        let cmd = ClientSubcommand::Restart {
            child: "restart".to_string(),
        };
        assert!(cmd.validate().is_err());
        assert_eq!(
            cmd.validate().err().unwrap().to_string(),
            "child name cannot be command"
        );

        let cmd2 = ClientSubcommand::Start {
            child: "stop".to_string(),
        };
        assert!(cmd2.validate().is_err());
    }

    #[test]
    fn test_client_all_subcommands_and_aliases() {
        // start
        let cmd = Command::new_from_str(vec!["start", "c1"]).unwrap();
        assert_eq!(cmd.op, Ops::Start);
        assert_eq!(cmd.child_name, Some("c1".to_string()));
        assert_eq!(cmd.as_bytes(), b"start c1");

        // Start (case)
        let cmd = Command::new_from_str(vec!["Start", "c1"]).unwrap();
        assert_eq!(cmd.op, Ops::Start);

        // stop
        let cmd = Command::new_from_str(vec!["stop", "c1"]).unwrap();
        assert_eq!(cmd.op, Ops::Stop);
        assert_eq!(cmd.child_name, Some("c1".to_string()));
        assert_eq!(cmd.as_bytes(), b"stop c1");

        // stop all
        let cmd = Command::new_from_str(vec!["stop", "all"]).unwrap();
        assert_eq!(cmd.op, Ops::Stop);
        assert_eq!(cmd.child_name, Some("all".to_string()));
        assert_eq!(cmd.as_bytes(), b"stop all");

        // trystart variations
        let cmd = Command::new_from_str(vec!["trystart", "c1"]).unwrap();
        assert_eq!(cmd.op, Ops::TryStart);
        assert_eq!(cmd.child_name, Some("c1".to_string()));
        assert_eq!(cmd.as_bytes(), b"trystart c1");

        let cmd = Command::new_from_str(vec!["Trystart", "c1"]).unwrap();
        assert_eq!(cmd.op, Ops::TryStart);

        let cmd = Command::new_from_str(vec!["TryStart", "c1"]).unwrap();
        assert_eq!(cmd.op, Ops::TryStart);

        // kill
        let cmd = Command::new_from_str(vec!["kill"]).unwrap();
        assert_eq!(cmd.op, Ops::Kill);
        assert_eq!(cmd.child_name, None);
        assert_eq!(cmd.as_bytes(), b"kill");

        // info
        let cmd = Command::new_from_str(vec!["info"]).unwrap();
        assert_eq!(cmd.op, Ops::Info);
        assert_eq!(cmd.child_name, None);
        assert_eq!(cmd.as_bytes(), b"info");

        let cmd = Command::new_from_str(vec!["info", "c1"]).unwrap();
        assert_eq!(cmd.op, Ops::Info);
        assert_eq!(cmd.child_name, Some("c1".to_string()));
        assert_eq!(cmd.as_bytes(), b"info c1");

        // check
        let cmd = Command::new_from_str(vec!["check"]).unwrap();
        assert_eq!(cmd.op, Ops::Check);
        assert_eq!(cmd.child_name, None);
        assert_eq!(cmd.as_bytes(), b"check");

        let cmd = Command::new_from_str(vec!["check", "c1"]).unwrap();
        assert_eq!(cmd.op, Ops::Check);
        assert_eq!(cmd.child_name, Some("c1".to_string()));
        assert_eq!(cmd.as_bytes(), b"check c1");
    }

    #[test]
    fn test_client_args_check_on_host() {
        // check on host without child name
        let cmd = Command::new_from_str(vec!["check", "on", "127.0.0.1"]).unwrap();
        assert_eq!(cmd.op, Ops::Check);
        assert_eq!(cmd.child_name, None);
        assert_eq!(cmd.prep, Some(vec![Prepositions::On]));
        assert_eq!(cmd.obj, Some(vec!["127.0.0.1".to_string()]));

        // check child on host
        let cmd = Command::new_from_str(vec!["check", "c1", "on", "127.0.0.1"]).unwrap();
        assert_eq!(cmd.op, Ops::Check);
        assert_eq!(cmd.child_name, Some("c1".to_string()));
        assert_eq!(cmd.prep, Some(vec![Prepositions::On]));
        assert_eq!(cmd.obj, Some(vec!["127.0.0.1".to_string()]));
    }

    #[test]
    fn test_client_multi_on_hosts() {
        let cmd = Command::new_from_str(vec![
            "restart", "c1", "on", "192.168.1.1", "on", "192.168.1.2",
        ]).unwrap();
        assert_eq!(cmd.op, Ops::Restart);
        assert_eq!(cmd.child_name, Some("c1".to_string()));
        assert_eq!(
            cmd.prep,
            Some(vec![Prepositions::On, Prepositions::On])
        );
        assert_eq!(
            cmd.obj,
            Some(vec!["192.168.1.1".to_string(), "192.168.1.2".to_string()])
        );
    }

    #[test]
    fn test_server_noise_args() {
        let args = ServerArgs::try_parse_from(&[
            "supervisor-rs-server",
            "--noise",
            "--noise-key",
            "/tmp/test.key",
            "--noise-authorized-keys",
            "/tmp/authorized_keys",
        ])
        .unwrap();
        assert!(args.noise);
        assert_eq!(args.noise_key, Some("/tmp/test.key".to_string()));
        assert_eq!(
            args.noise_authorized_keys,
            Some("/tmp/authorized_keys".to_string())
        );

        let args = ServerArgs::try_parse_from(&["supervisor-rs-server", "--keygen"]).unwrap();
        assert!(args.keygen);
    }

    #[test]
    fn test_client_noise_args() {
        let normalized = normalize_client_args(&[
            "supervisor-rs-client",
            "restart",
            "c1",
            "--noise",
            "-k",
            "/tmp/client.key",
            "--noise-authorized-keys",
            "/tmp/authorized_keys",
            "on",
            "127.0.0.1",
        ]);
        let client_args = ClientArgs::try_parse_from(normalized).unwrap();
        assert!(client_args.noise);
        assert_eq!(client_args.noise_key, Some("/tmp/client.key".to_string()));
        assert_eq!(
            client_args.noise_authorized_keys,
            Some("/tmp/authorized_keys".to_string())
        );
        assert_eq!(client_args.on, vec!["127.0.0.1".to_string()]);
    }

    #[test]
    fn test_client_keygen_subcommand() {
        let normalized = normalize_client_args(&["supervisor-rs-client", "keygen"]);
        let client_args = ClientArgs::try_parse_from(normalized).unwrap();
        assert_eq!(client_args.command, ClientSubcommand::Keygen { out: None });
        assert!(client_args.to_command().is_err());

        let normalized = normalize_client_args(&["supervisor-rs-client", "keygen", "my_key"]);
        let client_args = ClientArgs::try_parse_from(normalized).unwrap();
        assert_eq!(
            client_args.command,
            ClientSubcommand::Keygen {
                out: Some("my_key".to_string())
            }
        );
    }
}

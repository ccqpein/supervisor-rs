//! Legacy argument normalization and compatibility logic for 0.x CLI preposition syntax ("on <host>").
//!
//! To drop legacy 0.x CLI argument support in the future:
//! 1. Delete this file (`src/legacy_arg.rs`).
//! 2. Remove `pub mod legacy_arg;` from `src/lib.rs`.
//! 3. In `src/bin/client.rs`: call `ClientArgs::parse()` directly instead of `normalize_client_args`.
//! 4. In `src/client.rs`: pass arguments directly to `ClientArgs::try_parse_from`.

/// Normalize arguments to support 0.x preposition syntax ("on <host>")
/// by rewriting them to standard clap flags ("--on <host>").
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
    use crate::arg::{ClientArgs, ClientSubcommand};
    use crate::client::{Command, Ops, Prepositions};

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
        assert_eq!(cmd.prep, Some(vec![Prepositions::On]));
        assert_eq!(cmd.obj, Some(vec!["127.0.0.1".to_string()]));
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
        ])
        .unwrap();
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
}

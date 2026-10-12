use anyhow::{bail, Result};
use std::fmt;
use yaml_rust::Yaml;

/// Repeat struct including the action and seconds
#[derive(Debug, Clone)]
pub struct Repeat {
    pub action: String,
    pub seconds: i64,
}

impl Repeat {
    pub fn new_empty() -> Self {
        Repeat {
            action: String::from(""),
            seconds: 0,
        }
    }

    /// make new Repeat with yaml
    pub fn new(input: &Yaml) -> Result<Self> {
        let mut result = Repeat {
            action: String::from("restart"),
            seconds: 0,
        };

        let repeat = match input.as_hash() {
            Some(v) => v,
            None => {
                bail!("cannot found");
            }
        };

        if let Some(v) = repeat.get(&Yaml::from_str("action")) {
            if let Some(a) = v.clone().into_string() {
                result.action = a;
            }
        }

        match repeat.get(&Yaml::from_str("seconds")) {
            Some(v) => {
                if let Some(a) = v.clone().into_i64() {
                    result.seconds = a;
                }
            }
            None => {
                bail!("seconds cannot be empty");
            }
        };

        if result.seconds > 0 {
            Ok(result)
        } else {
            bail!("seconds cannot less or equal 0");
        }
    }
}

impl fmt::Display for Repeat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.seconds == 0 {
            write!(f, "false")
        } else {
            write!(
                f,
                "action is {}, interval is {} seconds",
                self.action, self.seconds
            )
        }
    }
}

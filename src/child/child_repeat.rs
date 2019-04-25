use std::io::{Error as ioError, ErrorKind, Result};
use yaml_rust::Yaml;

#[derive(Debug, Clone)]
pub struct Repeat {
    pub action: String,
    pub seconds: i64,
}

impl Repeat {
    pub fn new(input: &Yaml) -> Result<Self> {
        let mut result = Repeat {
            action: String::from("restart"),
            seconds: 0,
        };

        let repeat = match input.as_hash() {
            Some(v) => v,
            None => {
                return Err(ioError::new(ErrorKind::NotFound, format!("cannot found")));
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
                return Err(ioError::new(
                    ErrorKind::InvalidData,
                    format!("seconds cannot be empty"),
                ));
            }
        };

        if result.seconds > 0 {
            Ok(result)
        } else {
            Err(ioError::new(
                ErrorKind::InvalidData,
                format!("seconds cannot less or equal 0"),
            ))
        }
    }
}

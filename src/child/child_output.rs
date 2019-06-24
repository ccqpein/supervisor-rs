use std::fmt;
use std::io::{Error as ioError, ErrorKind, Result};
use yaml_rust::Yaml;

#[derive(Debug, Copy, Clone)]
pub enum OutputMode {
    Create,
    Append,
}

#[derive(Debug, Clone)]
pub struct Output {
    pub path: String,
    pub mode: OutputMode,
}

impl Output {
    pub fn new_empty() -> Self {
        Output {
            path: String::new(),
            mode: OutputMode::Create,
        }
    }

    pub fn new(input: Yaml) -> Result<Vec<(String, Self)>> {
        let lst = match input.into_vec() {
            Some(lst) => lst,
            None => {
                return Err(ioError::new(
                    ErrorKind::InvalidData,
                    format!("output format wrong"),
                ));
            }
        };

        let mut result = vec![];

        for hash in lst {
            let mut temp = (
                String::new(),
                Self {
                    path: String::new(),
                    mode: OutputMode::Create,
                },
            );
            for (p, m) in hash.into_hash().unwrap().iter() {
                match p.as_str() {
                    Some("mode") => match m.as_str() {
                        Some("create") => temp.1.mode = OutputMode::Create,
                        Some("append") => temp.1.mode = OutputMode::Append,
                        _ => (),
                    },
                    Some("stdout") => match m.as_str() {
                        Some(s) => {
                            temp.1.path = s.to_string();
                            temp.0 = "stdout".to_string()
                        }
                        None => {
                            return Err(ioError::new(
                                ErrorKind::InvalidData,
                                format!("stdout no path"),
                            ));
                        }
                    },
                    Some("stderr") => match m.as_str() {
                        Some(s) => {
                            temp.1.path = s.to_string();
                            temp.0 = "stderr".to_string()
                        }
                        None => {
                            return Err(ioError::new(
                                ErrorKind::InvalidData,
                                format!("stderr no path"),
                            ));
                        }
                    },
                    _ => {
                        return Err(ioError::new(
                            ErrorKind::InvalidData,
                            format!("output including illegal field"),
                        ));
                    }
                }
            }
            result.push(temp);
        }

        Ok(result)
    }
}

impl fmt::Display for Output {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.path == "" {
            return write!(f, "none");
        }
        write!(f, "path: {}, mode: {}", self.path, self.mode)
    }
}

impl fmt::Display for OutputMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Create => write!(f, "{}", "create"),
            Append => write!(f, "{}", "append"),
        }
    }
}

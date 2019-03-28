pub mod client;
pub mod kindergarten;
pub mod server;
pub mod timer;

use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{Error as ioError, ErrorKind, Read, Result};
use std::time;
use yaml_rust::{Yaml, YamlLoader};

#[derive(Debug, Copy, Clone)]
enum OutputMode {
    Create,
    Append,
}

#[derive(Debug, Clone)]
struct Output {
    path: String,
    mode: OutputMode,
}

impl Output {
    fn new(input: Yaml) -> Result<Vec<(String, Self)>> {
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

#[derive(Debug, Clone)]
pub struct Repeat {
    action: String,
    seconds: u64,
}

//:= TODO: need write repeat logic in doc
impl Repeat {
    fn new(input: &Yaml) -> Result<Self> {
        let mut result = Repeat {
            action: String::from("restart"),
            seconds: 0,
        };

        let repeat = match input.as_hash() {
            Some(v) => v,
            None => {
                return Err(ioError::new(
                    ErrorKind::InvalidData,
                    format!("output format wrong"),
                ));
            }
        };

        if let Some(v) = repeat.get(&Yaml::from_str("action")) {
            if let Some(a) = v.clone().into_string() {
                result.action = a;
            }
        }

        //:= TODO: 0 should not be legal interval, need check
        if let Some(v) = repeat.get(&Yaml::from_str("seconds")) {
            if let Some(a) = v.clone().into_i64() {
                result.seconds = a as u64;
            }
        }

        Ok(result)
    }
}

#[derive(Debug)]
pub struct Config {
    comm: String,
    stdout: Option<Output>,
    stderr: Option<Output>,
    child_id: Option<u32>,
    repeat: Option<Repeat>,
}

impl Config {
    fn new(comm: String) -> Self {
        Config {
            comm: comm,
            stdout: None,
            stderr: None,
            child_id: None,
            repeat: None,
        }
    }

    fn read_from_str(input: &str) -> Result<Self> {
        let temp = YamlLoader::load_from_str(input);

        let mut result: Self;
        match temp {
            Ok(docs) => {
                let doc = &docs[0];

                result = Self::new(doc["command"].clone().into_string().unwrap());

                if let Ok(output) = Output::new(doc["output"].clone()) {
                    for (field, data) in output {
                        if field == "stdout".to_string() {
                            result.stdout = Some(data);
                        } else if field == "stderr".to_string() {
                            result.stderr = Some(data);
                        }
                    }
                }

                result.repeat = if let Ok(r) = Repeat::new(&doc["repeat"]) {
                    Some(r)
                } else {
                    None
                }
            }

            Err(e) => return Err(ioError::new(ErrorKind::Other, e.description().to_string())),
        }

        Ok(result)
    }

    fn read_from_yaml_file(filename: &str) -> Result<Self> {
        let contents = File::open(filename);
        let mut string_result = String::new();
        match contents {
            Ok(mut cont) => {
                let _ = cont.read_to_string(&mut string_result);
                return Self::read_from_str(string_result.as_str());
            }

            Err(e) => return Err(ioError::new(ErrorKind::Other, e.description().to_string())),
        }
    }

    fn split_args(&self) -> (String, Option<String>) {
        let split_comm: Vec<_> = self.comm.splitn(2, ' ').collect();

        if split_comm.len() > 1 {
            return (split_comm[0].to_string(), Some(split_comm[1].to_string()));
        }

        (split_comm[0].to_string(), None)
    }

    fn is_repeat(&self) -> bool {
        if let Some(_) = self.repeat {
            return true;
        }
        false
    }

    fn to_duration(&self) -> Result<time::Duration> {
        if !self.is_repeat() {
            return Err(ioError::new(
                ErrorKind::Other,
                format!("do not find repeat value"),
            ));
        }

        match self.repeat.as_ref().unwrap().seconds.clone() {
            0 => Err(ioError::new(
                ErrorKind::Other,
                format!("repeat time cannot be 0"),
            )),
            d => Ok(time::Duration::from_secs(d)),
        }
    }

    fn repeat_command(&self) -> Result<String> {
        if !self.is_repeat() {
            return Err(ioError::new(
                ErrorKind::Other,
                format!("do not find repeat value"),
            ));
        }

        Ok(self.repeat.as_ref().unwrap().action.clone())
    }
}

impl Clone for Config {
    fn clone(&self) -> Self {
        Config {
            comm: self.comm.clone(),
            stdout: self.stdout.clone(),
            stderr: self.stderr.clone(),
            child_id: self.child_id,
            repeat: self.repeat.clone(),
        }
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "  command is: {}\n  stdout is: {:?}\n  stderr is: {:?}\n  child id is:{:?}\n  repeat is: {:?}",
            self.comm, self.stdout, self.stderr, self.child_id, self.repeat,            
        )
    }
}

// tiny tests below
#[cfg(test)]
mod tests {
    use super::*;
    use server::start_new_child;

    #[test]
    fn command_argvs() {
        let con = dbg!(Config::read_from_yaml_file("./test/argv.yml")).unwrap();
        let (comm, argvs) = con.split_args();
        println!("command: {}", comm);

        println!("{:?}", con.split_args());

        println!("{:?}", argvs.unwrap().split(' ').collect::<Vec<&str>>());
    }

    #[test]
    fn run_ls() {
        let mut con = dbg!(Config::read_from_yaml_file("./test/ls.yaml")).unwrap();

        let _ = dbg!(start_new_child(&mut con));
    }
}

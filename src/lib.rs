pub mod client;
pub mod kindergarten;
pub mod server;

use std::error::Error;
use std::fs::File;
use std::io::{Error as ioError, ErrorKind, Read, Result};
use yaml_rust::YamlLoader;

#[derive(Debug)]
pub struct Config {
    comm: String,
    stdout: Option<String>,
    stderr: Option<String>,
    child_id: Option<u32>,
}

impl Config {
    pub fn new(comm: String) -> Self {
        Config {
            comm: comm,
            stdout: None,
            stderr: None,
            child_id: None,
        }
    }

    pub fn new_stdout(mut self, stdout: String) -> Self {
        self.stdout = Some(stdout);
        self
    }

    pub fn new_stderr(mut self, stderr: String) -> Self {
        self.stderr = Some(stderr);
        self
    }

    pub fn read_from_str(input: &str) -> Result<Self> {
        let temp = YamlLoader::load_from_str(input);

        let mut result: Self;
        match temp {
            Ok(docs) => {
                let doc = &docs[0];

                result = Self::new(doc["Command"][0].clone().into_string().unwrap());

                if let Some(stdo) = doc["Stdout"][0].clone().into_string() {
                    result = result.new_stdout(stdo)
                }

                if let Some(stde) = doc["Stderr"][0].clone().into_string() {
                    result = result.new_stderr(stde);
                }
            }

            Err(e) => return Err(ioError::new(ErrorKind::Other, e.description().to_string())),
        }

        Ok(result)
    }

    pub fn read_from_yaml_file(filename: &str) -> Result<Self> {
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
}
impl Clone for Config {
    fn clone(&self) -> Self {
        Config {
            comm: self.comm.clone(),
            stdout: self.stdout.clone(),
            stderr: self.stderr.clone(),

            child_id: self.child_id,
        }
    }
}

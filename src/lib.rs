extern crate yaml_rust;

use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::process::Command;
use yaml_rust::{ScanError, YamlEmitter, YamlLoader};

#[derive(Debug)]
pub struct Config {
    Comm: String,
    Args: Option<Vec<String>>,
    Stdout: String,
}

impl Config {
    pub fn new(comm: String, args: Vec<String>, stdout: String) -> Self {
        Config {
            Comm: comm,
            Args: Some(args),
            Stdout: stdout,
        }
    }

    pub fn read_from_str(input: &str) -> Result<Self, String> {
        let temp = YamlLoader::load_from_str(input);
        match temp {
            Ok(docs) => {
                let doc = &docs[0];
                let comm = doc["Command"][0].clone();
                return Ok(Config {
                    Comm: comm.into_string().unwrap(),
                    Args: None,
                    Stdout: String::new(),
                });
            }
            Err(e) => return Err(e.description().to_string()),
        }
    }

    pub fn read_from_yaml_file(filename: &str) -> Result<Self, String> {
        let contents = File::open(filename);
        let mut string_result = String::new();
        match contents {
            Ok(mut cont) => {
                cont.read_to_string(&mut string_result);
                return Self::read_from_str(string_result.as_str());
            }

            Err(e) => return Err(e.description().to_string()),
        }
    }
}
//:= MARK: need find a way to store process id, and handle stdout
pub fn start_new_subprocessing(config: &Config) {
    let mut child = Command::new(&config.Comm)
        .args(config.Args.as_ref().unwrap())
        .stdout(File::create(&config.Stdout).unwrap())
        .spawn();
}

//:= MARK: take care all children in case they are stop running
fn watch_child() {}

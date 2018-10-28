extern crate yaml_rust;

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

    pub fn read_from_str(input: &'static str) -> Result<Self, ScanError> {
        if let Ok(docs) = YamlLoader::load_from_str(input) {
            let doc = &docs[0];
            let comm = doc["Command"][0].as_str().unwrap();
            return Ok(Config {
                Comm: comm.to_string(),
                Args: None,
                Stdout: String::new(),
            });
        }

        return Ok(Config {
            Comm: String::new(),
            Args: None,
            Stdout: String::new(),
        });
    }

    pub fn read_from_yaml() {}
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

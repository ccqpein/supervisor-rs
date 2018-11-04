use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::process::Command;
use yaml_rust::{ScanError, Yaml, YamlEmitter, YamlLoader};

#[derive(Debug)]
pub struct Config {
    comm: String,
    stdout: Option<String>,
    stderr: Option<String>,
}

impl Config {
    pub fn new(comm: String) -> Self {
        Config {
            comm: comm,
            stdout: None,
            stderr: None,
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

    //:= TODO: need more generic and match block
    pub fn read_from_str(input: &str) -> Result<Self, String> {
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

            Err(e) => return Err(e.description().to_string()),
        }

        Ok(result)
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

    fn split_args(&self) -> (String, Option<String>) {
        let split_comm: Vec<_> = self.comm.splitn(2, ' ').collect();

        if split_comm.len() > 1 {
            return (split_comm[0].to_string(), Some(split_comm[1].to_string()));
        }

        (split_comm[0].to_string(), None)
    }
}

//:= MARK: need find a way to store process id, and handle stdout
pub fn start_new_subprocessing(config: &Config) {
    let (com, args) = config.split_args();

    let mut child = Command::new(&com);

    match args {
        Some(arg) => {
            child.args(arg.split(' ').collect::<Vec<&str>>());
        }
        _ => (),
    }

    child
        .stdout(File::create(config.stdout.as_ref().unwrap()).unwrap())
        .spawn()
        .expect("Cannot start a new child processing");
}

//:= MARK: take care all children in case they are stop running
fn watch_child() {}

use std::fs::File;
use std::io::prelude::*;
use std::process::Command;

pub struct Config<'a> {
    Comm: &'a str,
    Args: &'a Vec<&'static str>,
    Stdout: &'a str,
}

impl<'a> Config<'a> {
    pub fn new(comm: &'a str, args: &'a Vec<&'static str>, stdout: &'a str) -> Self {
        Config {
            Comm: comm,
            Args: args,
            Stdout: stdout,
        }
    }
    pub fn read_from() {}
}
//:= MARK: need find a way to store process id, and handle stdout
pub fn start_new_subprocessing(config: &Config) {
    let mut child = Command::new(config.Comm)
        .args(config.Args)
        .stdout(File::create(config.Stdout).unwrap())
        .spawn();
}

//:= MARK: take care all children in case they are stop running
fn watch_child() {}

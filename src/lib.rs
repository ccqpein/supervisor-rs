use std::process::Command;

pub struct Config<'a> {
    Comm: &'a str,
    Args: &'a Vec<&'static str>,
    Stdout: &'a str,
}

impl<'a> Config<'a> {
    pub fn new() {}
    pub fn read_from() {}
}
//:= MARK: need find a way to store process id, and handle stdout
fn start_new_subprocessing(comm: &str, config: &Config) {
    let mut child = Command::new(config.Comm).args(config.Args);
}

//:= MARK: take care all children in case they are stop running
fn watch_child() {}

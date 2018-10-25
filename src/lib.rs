use std::process::Command;

struct Config<'a> {
    Comm: &'a str,
}

impl<'a> Config<'a> {
    fn new() {}
    fn read_from() {}
}
//:= MARK: need find a way to store process id, and handle stdout
fn start_new_subprocessing(comm: &str, config: &Config) {}

//:= MARK: take care all children in case they are stop running
fn watch_child() {}

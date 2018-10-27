//extern crate supervisor_rs;
use supervisor_rs::*;

fn main() {
    let temp = vec!["run", "~/Desktop/main.go"];
    let a = Config::new("go", &temp, "/tmp/log");
    start_new_subprocessing(&a);
    println!("Hello, world!");
    loop {}
}

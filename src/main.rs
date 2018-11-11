use std::{thread, time};
use supervisor_rs::*;

fn main() {
    let mut conf = Config::read_from_yaml_file("/tmp/test.yml").unwrap();
    println!("{:?}", conf);

    let a = start_new_child(&mut conf);
    thread::sleep_ms(2000);

    if let Ok(mut c) = a {
        println!("{}", c.id());
        c.kill().expect("command wasn't running");
        //c.kill().expect("command wasn't running");
    };
    loop {}
}

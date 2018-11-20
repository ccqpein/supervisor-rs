use self::server::start_new_child;
use std::thread;
use supervisor_rs::*;

fn main() {
    //let mut server_conf = Config::read_from_yaml_file("/tmp/test.yml").unwrap();

    let mut conf = Config::read_from_yaml_file("/tmp/test.yml").unwrap();
    println!("{:?}", conf);

    let a = start_new_child(&mut conf);
    //thread::spawn(|| loop {
    //    day_care();
    //});
    loop {}
}

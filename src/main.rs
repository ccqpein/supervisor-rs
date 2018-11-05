//extern crate supervisor_rs;
use supervisor_rs::*;

fn main() {
    let c = Config::read_from_yaml_file("/tmp/test.yml").unwrap();
    println!("{:?}", c);

    let a = start_new_child(&c).unwrap();
    println!("{}", a.id());
    loop {}
}

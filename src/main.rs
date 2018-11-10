//extern crate supervisor_rs;
use supervisor_rs::*;

fn main() {
    let mut c = Config::read_from_yaml_file("/tmp/test.yml").unwrap();
    println!("{:?}", c);

    let a = start_new_child(&mut c).unwrap();
    println!("{}", a.id());
    loop {}
}

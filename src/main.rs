//extern crate supervisor_rs;
use supervisor_rs::*;

fn main() {
    let c = Config::read_from_yaml_file("/tmp/test.yml").unwrap();
    println!("{:?}", c);

    //start_new_subprocessing(&c);
    loop {}
}

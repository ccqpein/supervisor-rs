//extern crate supervisor_rs;
use supervisor_rs::*;

fn main() {
    let mut conf = Config::read_from_yaml_file("/tmp/test.yml").unwrap();
    println!("{:?}", conf);

    let a = start_new_child(&mut conf);

    if let Ok(mut c) = a {
        println!("{}", c.id());
        //c.kill().expect("command wasn't running");
        println!("{:?}", c.stdout);
        println!("{:?}", c.stderr);
    };
    loop {}
}

//extern crate supervisor_rs;
use supervisor_rs::*;

fn main() {
    let bs = "
Command:
    - go
Args:
    - aa
";
    let b = Config::read_from_str(bs).unwrap();
    println!("{:?}", b);
    loop {}
}

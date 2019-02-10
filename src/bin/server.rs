use std::env;
use std::io::Result;
use supervisor_rs::server;

fn main() -> Result<()> {
    let arguments = env::args();
    let change_2_vec = arguments.collect::<Vec<String>>();

    if change_2_vec.len() > 2 {
        println!("{}", "too much arguments, not support yet.");
        return Ok(());
    }

    let k = if change_2_vec.len() != 1 {
        server::start_new_server(&change_2_vec[1])?
    } else {
        server::start_new_server("")?
    };

    server::start_deamon(k)
}

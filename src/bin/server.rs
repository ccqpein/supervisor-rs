use std::env;
use std::io::Result;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
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

    //make channel for deamon & main communication
    let (tx, rx) = mpsc::channel();

    //give thread safe kindergarden here
    let kg = Arc::new(Mutex::new(k));

    //use an additional thread to handle deamon, and send message out.
    let _ = thread::spawn(move || server::start_deamon(kg, tx));

    //handle message
    for (f, _) in rx {
        if f == "I am dying. " {
            println!("see you!");
            return Ok(());
        }
    }

    Ok(())
}

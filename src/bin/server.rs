use clap::Parser;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use supervisor_rs::arg::ServerArgs;
use supervisor_rs::logger;
use supervisor_rs::server;

fn main() {
    let args = ServerArgs::parse();

    let k_result = server::start_new_server(args.config_path());

    let k = match k_result {
        Ok(k) => k,
        Err(e) => {
            println!("{}", logger::timelog(&e.to_string()));
            return;
        }
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
            return;
        }
    }

    ()
}

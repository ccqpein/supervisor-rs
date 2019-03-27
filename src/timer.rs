use super::kindergarten::*;
use super::server;
use super::*;
use std::io::Error as ioError;
use std::{thread, time};

use std::sync::{Arc, Mutex};

//:= TODO: need check in case kg and timer not sync, maybe
pub struct timer {
    name: String,
    id: u32,
    comm: String,
    interval: time::Duration,
}

impl timer {
    pub fn new(name: String, id: u32, td: time::Duration) -> Self {
        timer {
            name: name.clone(),
            id: id,
            comm: String::new(),
            interval: td,
        }
    }

    pub fn run(self, kig: Arc<Mutex<Kindergarten>>, comm: String) {
        thread::sleep(self.interval);
        if let Err(e) = server::day_care(kig, comm.clone()) {
            println!("timer is up, but {:?}", e);
        } else {
            println!("timer is up, {}", comm);
        }
    }
}

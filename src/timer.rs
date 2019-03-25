use super::kindergarten::*;
use super::server;
use super::*;
use std::io::Error as ioError;
use std::{thread, time};

use std::sync::{Arc, Mutex};

pub struct timer {
    name: String,
    id: u32,
    interval: time::Duration,
}

impl timer {
    pub fn new(name: &String, id: u32, td: time::Duration) -> Self {
        timer {
            name: name.clone(),
            id: id,
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

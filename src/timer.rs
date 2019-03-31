use super::kindergarten::*;
use super::server;
use super::*;
use std::io::Error as ioError;
use std::{thread, time};

use std::sync::{Arc, Mutex};

pub struct Timer {
    name: String,
    id: u32,
    comm: String,
    interval: time::Duration,
}

impl Timer {
    fn new(name: &String, id: u32, comm: String, td: time::Duration) -> Self {
        Timer {
            name: name.clone(),
            id: id,
            comm: comm,
            interval: td,
        }
    }

    pub fn new_from_conf(name: String, conf: super::Config) -> Result<Self> {
        if !conf.is_repeat() {
            return Err(ioError::new(
                ErrorKind::InvalidInput,
                format!("config is not repeatable"),
            ));
        }

        Ok(Self::new(
            &name,
            conf.child_id.unwrap(),
            format!("{}", conf.repeat_command().unwrap()),
            conf.to_duration().unwrap(),
        ))
    }

    pub fn run(self, kig: Arc<Mutex<Kindergarten>>) {
        thread::sleep(self.interval);
        //check if this timer still works
        if !self.check(kig.clone()) {
            println!(
                "check failed when timer try to run \"{} {}\"",
                self.comm.clone(),
                self.name.clone()
            );
            return;
        }

        match server::day_care(kig, format!("{} {}", self.comm.clone(), self.name.clone())) {
            Err(e) => println!("Timer is up, but {:?}", e.description()),
            Ok(m) => println!(
                "Timer is up, run \"{} {}\"\n{}",
                self.comm.clone(),
                self.name.clone(),
                m,
            ),
        }
    }

    fn check(&self, kig: Arc<Mutex<Kindergarten>>) -> bool {
        let mut kg = kig.lock().unwrap();
        //if timer.id not equal child.id, means child has already restarted
        //then this timer is outdate
        if let Some(id) = kg.has_child(&self.name) {
            if *id == self.id {
                return true;
            }
        }

        return false;
    }
}

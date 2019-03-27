use super::kindergarten::*;
use super::server;
use super::*;
use std::io::Error as ioError;
use std::{thread, time};

use std::sync::{Arc, Mutex};

//:= TODO: need to have ability to terminate timer, in case in infinity loop
//:= TODO: need check in case kg and timer not sync, maybe
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
        if let Err(e) =
            server::day_care(kig, format!("{} {}", self.comm.clone(), self.name.clone()))
        {
            println!("Timer is up, but {:?}", e);
        } else {
            println!(
                "Timer is up, run {} {}",
                self.comm.clone(),
                self.name.clone()
            );
        }
    }

    //:= TODO: this should use name and id store in timer to check if KG is
    //fn check(){}
}

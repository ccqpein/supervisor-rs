use super::kindergarten::*;
use std::io::{Error as ioError, ErrorKind, Read, Result, Write};
use std::{thread, time};

pub fn timer_fn<F>(interval: u64, func: F) -> Result<String>
where
    F: Fn() -> Result<String>,
{
    let sleep_time_seconds = time::Duration::from_secs(interval);
    thread::sleep(sleep_time_seconds);
    func()
}

//timer_fn(5,day_care(kg))

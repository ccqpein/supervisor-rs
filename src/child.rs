mod child_hook;
pub mod child_output;
mod child_repeat;

use super::logger;
use chrono::prelude::*;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{Error as ioError, ErrorKind, Read, Result};
use std::time;
use yaml_rust::YamlLoader;

use child_hook::Hooks;
use child_output::Output;
use child_repeat::Repeat;

#[derive(Debug)]
pub struct Config {
    comm: String,
    pub stdout: Option<Output>,
    pub stderr: Option<Output>,
    repeat: Option<Repeat>,
    hooks: Option<Hooks>,

    pub child_id: Option<u32>,
    pub start_time: Option<DateTime<Local>>,
}

impl Config {
    pub fn new(comm: String) -> Self {
        Config {
            comm: comm,
            stdout: None,
            stderr: None,
            child_id: None,
            repeat: None,
            hooks: None,
            start_time: None,
        }
    }

    fn read_from_str(input: &str) -> Result<Self> {
        let temp = YamlLoader::load_from_str(input);

        let mut result: Self;
        match temp {
            Ok(docs) => {
                let doc = &docs[0];

                result = Self::new(doc["command"].clone().into_string().unwrap());

                if let Ok(output) = Output::new(doc["output"].clone()) {
                    for (field, data) in output {
                        if field == "stdout".to_string() {
                            result.stdout = Some(data);
                        } else if field == "stderr".to_string() {
                            result.stderr = Some(data);
                        }
                    }
                }

                result.repeat = match Repeat::new(&doc["repeat"]) {
                    Ok(r) => Some(r),
                    Err(e) => {
                        if e.kind() != ErrorKind::NotFound {
                            println!("{}", logger::timelog(e.description()));
                        }
                        None
                    }
                };

                result.hooks = match Hooks::new(&doc["hooks"]) {
                    Ok(h) => Some(h),
                    Err(e) => {
                        if e.kind() != ErrorKind::NotFound {
                            println!("{}", logger::timelog(e.description()));
                        }
                        None
                    }
                }
            }

            Err(e) => return Err(ioError::new(ErrorKind::Other, e.description().to_string())),
        }

        Ok(result)
    }

    pub fn read_from_yaml_file(filename: &str) -> Result<Self> {
        let contents = File::open(filename);
        let mut string_result = String::new();
        match contents {
            Ok(mut cont) => {
                let _ = cont.read_to_string(&mut string_result);
                return Self::read_from_str(string_result.as_str());
            }

            Err(e) => return Err(ioError::new(ErrorKind::Other, e.description().to_string())),
        }
    }

    pub fn split_args(&self) -> (String, Option<String>) {
        let split_comm: Vec<_> = self.comm.splitn(2, ' ').collect();

        if split_comm.len() > 1 {
            return (split_comm[0].to_string(), Some(split_comm[1].to_string()));
        }

        (split_comm[0].to_string(), None)
    }

    pub fn is_repeat(&self) -> bool {
        if let Some(_) = self.repeat {
            return true;
        }
        false
    }

    pub fn to_duration(&self) -> Result<time::Duration> {
        if !self.is_repeat() {
            return Err(ioError::new(
                ErrorKind::Other,
                format!("do not find repeat value"),
            ));
        }

        match self.repeat.as_ref().unwrap().seconds.clone() {
            0 => Err(ioError::new(
                ErrorKind::Other,
                format!("repeat time cannot be 0"),
            )),
            d => Ok(time::Duration::from_secs(d as u64)),
        }
    }

    pub fn repeat_command(&self) -> Result<String> {
        if !self.is_repeat() {
            return Err(ioError::new(
                ErrorKind::Other,
                format!("do not find repeat value"),
            ));
        }

        Ok(self.repeat.as_ref().unwrap().action.clone())
    }

    pub fn has_hook(&self) -> bool {
        if let Some(h) = &self.hooks {
            return h.has_hook();
        }
        false
    }

    pub fn get_hook(&self, key: &String) -> Option<String> {
        if self.has_hook() {
            return Some(
                self.hooks
                    .as_ref()
                    .unwrap()
                    .get(key)
                    .as_ref()
                    .unwrap()
                    .to_string(),
            );
        }

        None
    }

    pub fn get_hook_command(&self, key: &String) -> Option<String> {
        if self.has_hook() {
            return self.hooks.as_ref().unwrap().get_hook_command(key);
        }

        None
    }

    pub fn get_hook_detail(&self, key: &String) -> Option<Vec<String>> {
        if self.has_hook() {
            return self.hooks.as_ref().unwrap().get_hook_detail(key);
        }
        None
    }
}

impl Clone for Config {
    fn clone(&self) -> Self {
        Config {
            comm: self.comm.clone(),
            stdout: self.stdout.clone(),
            stderr: self.stderr.clone(),
            child_id: self.child_id,
            repeat: self.repeat.clone(),
            hooks: self.hooks.clone(),
            start_time: self.start_time.clone(),
        }
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "  command is: {}\n  stdout is: {}\n  stderr is: {}\n  child id is: {}\n  start time: {:?}\n  repeat is: {}\n  hooks are:\n{}",
            self.comm,
            self.stdout.as_ref().unwrap_or(&Output::new_empty()),
            self.stderr.as_ref().unwrap_or(&Output::new_empty()),
            self.child_id.as_ref().unwrap_or(&(0 as u32)),
            {if let Some(t) = self.start_time{
                t.format("%Y-%m-%d %H:%M:%S").to_string()
            }else {
                String::from("none")
            }},
            //Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            self.repeat.as_ref().unwrap_or(&Repeat::new_empty()),
            self.hooks.as_ref().unwrap_or(&Hooks::new_empty())
        )
    }
}

// tiny tests below
#[cfg(test)]
mod tests {
    use super::super::server::start_new_child;
    use super::*;

    //#[test]
    fn command_argvs() {
        let con = dbg!(Config::read_from_yaml_file("./test/argv.yml")).unwrap();
        let (comm, argvs) = con.split_args();
        println!("command: {}", comm);

        println!("{:?}", con.split_args());

        println!("{:?}", argvs.unwrap().split(' ').collect::<Vec<&str>>());
    }

    //#[test]
    fn run_ls() {
        let mut con = dbg!(Config::read_from_yaml_file("./test/ls.yaml")).unwrap();

        let _ = dbg!(start_new_child(&mut con));
    }

    //#[test]
    fn read_hooks() {
        let input0 = "
command: test
hooks:
  - prehook: start child1
  - posthook: start child2
  - posthook: start child3
";
        println!("read_hooks 0: {:?}", Config::read_from_str(input0));

        let input1 = "
command: test
hooks:
";

        println!("read_hooks 1: {:?}", Config::read_from_str(input1));
    }

    #[test]
    fn test_printout_config() {
        let input0 = "
command: test
output:
  - stdout: aaaaaa
    mode: append
hooks:
  - prehook: start child1
  - posthook: start child2
  - posthook: start child3
repeat:
  action: restart
  seconds: 5
";
        let conf = Config::read_from_str(input0).unwrap();

        println!("whole config is:\n{}", conf);
    }

}

use super::child::Config;
use super::logger;
use super::server::*;
use std::collections::HashMap;
use std::io::{Error as ioError, ErrorKind, Result};
use std::process::Child;

#[derive(Debug)]
pub struct Kindergarten {
    /// Store the path of server config
    pub server_config_path: String,

    /// child_id -> (child_handle, this child's config)
    id_list: HashMap<u32, (Child, Config)>,

    /// child_name -> child_id
    /// cannot accept duplicated name
    name_list: HashMap<String, u32>,

    /// encrypt mode
    pub encrypt_mode: bool,
}

impl Kindergarten {
    pub fn new() -> Self {
        Kindergarten {
            server_config_path: "".to_string(),
            id_list: HashMap::new(),
            name_list: HashMap::new(),

            encrypt_mode: false,
        }
    }

    /// register id_list, because id_list is a hashmap, so it if kinds of update too
    pub fn register_id(&mut self, id: u32, child: Child, config: Config) {
        self.id_list.insert(id, (child, config));
    }

    /// register name_list, because name_list is a hashmap, so it if kinds of update too
    pub fn register_name(&mut self, name: &String, id: u32) {
        self.name_list.insert(name.clone(), id);
    }

    /// update name_list and id_list
    fn update(&mut self, id: u32, name: &String, child: Child, config: Config) {
        self.register_id(id, child, config);
        self.register_name(name, id);
    }

    /// pre_hook_chain is Vec<(command, name, config)>
    /// if one of them failed in processing, this function will return without run commands after it.
    pub fn handle_pre_hook(&mut self, pre_hook_chain: Vec<(String, String, Config)>) -> Result<()> {
        for each in pre_hook_chain.iter() {
            match each.0.as_ref() {
                "start" | "Start" => self.start(&each.1, &mut each.2.clone())?,
                "restart" | "Restart" => self.restart(&each.1, &mut each.2.clone())?,
                "stop" | "Stop" => self.stop(&each.1)?,
                _ => (),
            };
        }
        Ok(())
    }

    /// start child.
    pub fn start(&mut self, name: &String, config: &mut Config) -> Result<()> {
        // check inside again (because "start" in server has checked once) here...
        // ...because prehook need check too, but it does not check in server
        if let Some(_) = self.has_child(name) {
            return Err(ioError::new(
                ErrorKind::InvalidData,
                format!("Cannot start child {}, it already exist.", name),
            ));
        };

        // start new child
        match start_new_child(config) {
            Ok(child) => {
                //update kindergarten
                let new_id = child.id();
                self.update(new_id, name, child, config.clone());
                Ok(())
            }
            Err(e) => {
                println!("{:?}", e);
                return Err(ioError::new(
                    ErrorKind::InvalidData,
                    format!("Cannot start child {}", name),
                ));
            }
        }
    }

    /// receive new config instead of read from kindergarten because maybe config change
    /// child which restart must be running child, so it can stop first
    ///
    /// Step:
    /// 1. kill old one
    /// 2. start new one
    /// 3. update kindergarten
    pub fn restart(&mut self, name: &String, config: &mut Config) -> Result<()> {
        // if this child is not running, it cannot be stopped, return err
        self.stop(name)?;

        // start new child
        self.start(name, config)
    }

    /// stop child, and delete it in kg, after this method, do not need delete child
    pub fn stop(&mut self, name: &String) -> Result<()> {
        // if stop all
        if name == "all" {
            return self.stop_all();
        } else if name == "" {
            return Err(ioError::new(
                ErrorKind::InvalidData,
                format!("you have to give which child you want to stop"),
            ));
        }

        // get id
        let id = match self.name_list.get(name).as_ref() {
            Some(id) => id,
            None => &1,
        };

        // check if this name of child in kindergarden
        if *id == 1 {
            return Err(ioError::new(
                ErrorKind::InvalidData,
                format!("{} not exsit, cannot stop", name),
            ));
        }

        // get child_handle
        let store_val = self.id_list.get_mut(&id).unwrap();
        let child_handle = &mut (store_val.0);

        // kill old child
        if let Err(e) = child_handle.kill() {
            println!("{:?}", e);
            return Err(ioError::new(
                ErrorKind::InvalidData,
                format!("Cannot kill child {}, id is {}, err is {}", name, id, e),
            ));
        }

        match child_handle.wait() {
            Ok(_) => {
                self.delete_by_name(name)?;
                Ok(())
            }
            Err(e) => Err(ioError::new(
                ErrorKind::InvalidData,
                format!("Cannot kill child {}, id is {}, err is {}", name, id, e),
            )),
        }
    }

    /// stop all children
    pub fn stop_all(&mut self) -> Result<()> {
        let names =
            { self.name_list.keys().into_iter().map(|x| x.clone()) }.collect::<Vec<String>>();

        for name in names {
            self.stop(&name)?;
        }

        Ok(())
    }

    /// check if some command have done already, clean them
    /// only return error if child_handle try_wait has problem
    pub fn check_around(&mut self) -> Result<()> {
        // this guard check for name_list and id_list aren't has same number
        // it shall not happen
        if self.name_list.len() != self.id_list.len() {
            return Err(ioError::new(
                ErrorKind::InvalidData,
                format!(
                    "number of name_list not match id_list, something wrong: \n{:#?}\n{:#?}",
                    self.name_list, self.id_list
                ),
            ));
        }

        let mut cache: Vec<String> = vec![];
        for (name, id) in self.name_list.iter() {
            let store_val = self.id_list.get_mut(id).unwrap();
            let child_handle = &mut (store_val.0);

            // try to find those children dead
            match child_handle.try_wait()? {
                Some(_) => {
                    let _ = child_handle.wait();
                    cache.push(name.clone());
                }
                None => (),
            }
        }

        for name in cache {
            self.delete_by_name(&name)?;
            println!(
                "{}",
                logger::timelog(&format!("{} has stopped, delete from kindergarden", name))
            );
        }

        Ok(())
    }

    /// delete by name, won't return error if no name
    pub fn delete_by_name(&mut self, name: &String) -> Result<()> {
        if let Some(id) = self.name_list.remove(name) {
            self.id_list.remove(&id);
        }

        Ok(())
    }

    /// check if kindergarten has this child, return option child id
    pub fn has_child(&mut self, name: &String) -> Option<&u32> {
        self.name_list.get(name)
    }

    /// get child's config by its name
    pub fn get_child_config(&mut self, name: &String) -> Option<Config> {
        let id = if let Some(id) = self.has_child(name) {
            id.clone()
        } else {
            return None;
        };

        Some(self.id_list.get(&id).as_ref().unwrap().1.clone())
    }

    /// handler command "check"
    pub fn check_status(&mut self, name: &String) -> Result<String> {
        // first check_around
        self.check_around()?;

        let mut res = String::from("==Check Results Below==\n");
        if name == "" {
            for (name, id) in self.name_list.iter() {
                res.push_str(&format!(
                    "child name: {}
processing id: {}
config detail:
{}
=======================\n",
                    name,
                    id,
                    self.id_list.get(id).unwrap().1
                ));
            }
        } else {
            if let Some(id) = self.name_list.get(name) {
                res.push_str(&format!(
                    "child name: {}
processing id: {}
config detail:
{}
=======================\n",
                    name,
                    id,
                    self.id_list.get(id).unwrap().1
                ))
            }
        }

        if res.is_empty() {
            Ok(String::from("check empty"))
        } else {
            Ok(res)
        }
    }
}

use super::server::*;
use super::Config;
use std::collections::HashMap;
use std::io::{Error as ioError, ErrorKind, Result};
use std::process::Child;

#[derive(Debug)]
pub struct Kindergarten {
    // store where is server config
    pub server_config_path: String,

    //child_id -> (child_handle, this child's config)
    id_list: HashMap<u32, (Child, Config)>,

    //child_name -> child_id
    name_list: HashMap<String, u32>,
}

impl Kindergarten {
    pub fn new() -> Self {
        Kindergarten {
            server_config_path: "".to_string(),
            id_list: HashMap::new(),
            name_list: HashMap::new(),
        }
    }

    pub fn register_id(&mut self, id: u32, child: Child, config: Config) {
        self.id_list.insert(id, (child, config));
    }

    pub fn register_name(&mut self, name: &String, id: u32) {
        self.name_list.insert(name.clone(), id);
    }

    //update
    pub fn update(&mut self, id: u32, name: &String, child: Child, config: Config) {
        self.register_id(id, child, config);
        self.register_name(name, id);
    }

    //receive new config instead of read from kindergarten because maybe config change
    //child which restart must be running child, so it can stop first
    //Step:
    //1. kill old one
    //2. start new one
    //3. update kindergarten
    pub fn restart(&mut self, name: &String, config: &mut Config) -> Result<()> {
        //if this child is not running, it cannot be stopped, return err
        self.stop(name)?;

        //start new child
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

    //stop child, and delete it in kg, after this method, do not need delete child
    pub fn stop(&mut self, name: &String) -> Result<()> {
        //if stop all
        if name == "all" {
            return self.stop_all();
        }

        //get id
        let id = match self.name_list.get(name).as_ref() {
            Some(id) => id,
            None => &1,
        };

        //check if this name of child in kindergarden
        if *id == 1 {
            return Err(ioError::new(
                ErrorKind::InvalidData,
                format!("{} not exsit, cannot stop", name),
            ));
        }

        //get child_handle
        let store_val = self.id_list.get_mut(&id).unwrap();
        let child_handle = &mut (store_val.0);

        //kill old child
        if let Err(e) = child_handle.kill() {
            println!("{:?}", e);
            return Err(ioError::new(
                ErrorKind::InvalidData,
                format!("Cannot kill child {}, id is {}, err is {}", name, id, e),
            ));
        }

        self.delete_by_name(name)?;

        Ok(())
    }

    //stop all children
    pub fn stop_all(&mut self) -> Result<()> {
        let names =
            { self.name_list.keys().into_iter().map(|x| x.clone()) }.collect::<Vec<String>>();

        for name in names {
            self.stop(&name)?;
        }

        Ok(())
    }

    //check if some command have done already, clean them
    //only return error if child_handle try_wait has problem
    pub fn check_around(&mut self) -> Result<()> {
        let mut cache: Vec<String> = vec![];
        for (name, id) in self.name_list.iter() {
            let store_val = self.id_list.get_mut(id).unwrap();
            let child_handle = &mut (store_val.0);

            match child_handle.try_wait()? {
                Some(_) => {
                    cache.push(name.clone());
                }
                None => (),
            }
        }

        for name in cache {
            self.delete_by_name(&name)?;
            println!("{} has stopped, delete from kindergarden", name);
        }

        Ok(())
    }

    //delete by name, won't return error if no name
    pub fn delete_by_name(&mut self, name: &String) -> Result<()> {
        if let Some(id) = self.name_list.remove(name) {
            self.id_list.remove(&id);
        }

        Ok(())
    }

    pub fn has_child(&mut self, name: &String) -> Option<&u32> {
        self.name_list.get(name)
    }
}

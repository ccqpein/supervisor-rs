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

    //Step:
    //1. kill old one
    //2. start new one
    //3. update kindergarten
    pub fn restart(&mut self, name: &String, config: &mut Config) -> Result<()> {
        //get id
        let id = self.name_list.get(name).unwrap();
        //get child_handle
        let store_val = self.id_list.get_mut(&id).unwrap();
        let child_handle = &mut (store_val.0);

        //kill old child
        if let Err(e) = child_handle.kill() {
            println!("{:?}", e);
            return Err(ioError::new(
                ErrorKind::InvalidData,
                format!("Cannot kill child {}, id is {}", name, id),
            ));
        }

        //start new child
        match start_new_child(config) {
            Ok(child) => {
                //update kindergarten
                let new_id = child.id();
                //remove old id to make sure one-to-one relationship
                self.id_list.remove(&id);
                self.update(new_id, name, child, config.clone());
                Ok(())
            }
            Err(e) => {
                println!("{:?}", e);
                return Err(ioError::new(
                    ErrorKind::InvalidData,
                    format!("Cannot kill child {}, id is {}", name, id),
                ));
            }
        }
    }

    pub fn stop(&mut self, name: &String) -> Result<()> {
        //get id
        let id = self.name_list.get(name).unwrap();
        //get child_handle
        let store_val = self.id_list.get_mut(&id).unwrap();
        let child_handle = &mut (store_val.0);

        //kill old child
        if let Err(e) = child_handle.kill() {
            println!("{:?}", e);
            return Err(ioError::new(
                ErrorKind::InvalidData,
                format!("Cannot kill child {}, id is {}", name, id),
            ));
        }

        //clean Kindergarten
        self.id_list.remove(id);
        self.name_list.remove(name);

        Ok(())
    }
}
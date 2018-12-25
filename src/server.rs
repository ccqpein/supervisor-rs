use super::client;
use super::Config;

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{Error as ioError, ErrorKind, Read, Result};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use yaml_rust::YamlLoader;

#[derive(Debug)]
struct ServerConfig {
    //path for all children's configs
    load_path: String,
}

impl ServerConfig {
    fn load(filename: &str) -> Result<Self> {
        let mut contents = File::open(filename)?;
        let mut string_result = String::new();

        contents.read_to_string(&mut string_result)?;
        Self::read_from_str(string_result.as_str())
    }

    fn read_from_str(input: &str) -> Result<Self> {
        let temp = YamlLoader::load_from_str(input);
        let mut result: Self;

        match temp {
            Ok(docs) => {
                let doc = &docs[0];
                result = ServerConfig {
                    load_path: doc["loadpath"][0].clone().into_string().unwrap(),
                }
            }
            Err(e) => return Err(ioError::new(ErrorKind::Other, e.description().to_string())),
        }

        Ok(result)
    }

    //return vec of (filename, path)
    fn all_ymls_in_load_path(&self) -> Result<Vec<(String, String)>> {
        let mut result: Vec<(String, String)> = vec![];
        for entry in fs::read_dir(self.load_path.clone())? {
            if let Ok(entry) = entry {
                if let Some(extension) = entry.path().extension() {
                    if extension == "yml" {
                        result.push((
                            entry
                                .file_name()
                                .to_str()
                                .unwrap()
                                .split('.')
                                .collect::<Vec<&str>>()[0]
                                .to_string(),
                            entry.path().to_str().unwrap().to_string(),
                        ));
                    }
                }
            }
        }
        Ok(result)
    }

    //return whole path of file which match filename
    fn find_config_by_name(&self, filename: &String) -> Result<Config> {
        for entry in fs::read_dir(self.load_path.clone())? {
            if let Ok(entry) = entry {
                if let Some(extension) = entry.path().extension() {
                    if extension == "yml" {
                        if entry
                            .file_name()
                            .to_str()
                            .unwrap()
                            .split('.')
                            .collect::<Vec<&str>>()[0]
                            .to_string()
                            == *filename
                        {
                            return Config::read_from_yaml_file(entry.path().to_str().unwrap());
                        }
                    }
                }
            }
        }

        Err(ioError::new(
            ErrorKind::NotFound,
            format!("Cannot found this file in load path"),
        ))
    }
}

#[derive(Debug)]
pub struct Kindergarten {
    id_list: HashMap<u32, (Child, Config)>,
    name_list: HashMap<String, u32>,
}

impl Kindergarten {
    pub fn new() -> Self {
        Kindergarten {
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
}

//start a child processing, and give child_handle
//side effection: config.child_id be updated
pub fn start_new_child(config: &mut Config) -> Result<Child> {
    let (com, args) = config.split_args();

    let mut command = Command::new(&com);

    match args {
        Some(arg) => {
            command.args(arg.split(' ').collect::<Vec<&str>>());
        }
        _ => (),
    };

    //run command and give child handle
    let child = command
        .stdout(File::create(config.stdout.as_ref().unwrap()).unwrap())
        .spawn();

    match child {
        Ok(ref c) => {
            config.child_id = Some(c.id());
            return child;
        }
        _ => return child, //:= TODO: error handler
    };
}

//for deamon to start new child
//receive file path, make it to config, then start it
//return id, config for deamon to update kindergarten
pub fn start_new_child_with_file(filepath: &str) -> Result<(u32, Config)> {
    let mut conf = Config::read_from_yaml_file(filepath)?;

    start_new_child(&mut conf)?;

    return Ok((conf.child_id.unwrap(), conf));
}

//Receive server config and start a new server
//new server including:
//1. a way receive command from client //move to start_deamon
//2. first start will start all children in config path
//3. then keep listening commands and can restart each of them //move to start deamon
pub fn start_new_server() -> Result<Kindergarten> {
    //Read server's config file
    let server_conf = ServerConfig::load("/tmp/server.yml")?;

    //create new kindergarten
    let mut kindergarten = Kindergarten::new();

    //run all config already in load path
    for conf in server_conf.all_ymls_in_load_path()? {
        let mut child_config = Config::read_from_yaml_file(&conf.1)?;

        let child_handle = start_new_child(&mut child_config)?;
        //registe id
        let id = child_config.child_id.unwrap();
        kindergarten.register_id(id, child_handle, child_config);
        //regist name
        kindergarten.register_name(&conf.0, id);
    }

    Ok(kindergarten)
}

//check all children are fine or not
//if not fine, try to restart them
//need channel input to update kindergarten
fn day_care(mut kg: Kindergarten, rec: Receiver<String>) {
    loop {
        //println!("{:#?}", kg);
        let data = rec.recv().unwrap();
        let command = client::Command::new_from_string(data);

        match command.op {
            client::Ops::Restart => {
                let server_conf = ServerConfig::load("/tmp/server.yml").unwrap();

                match server_conf.find_config_by_name(command.child_name.as_ref().unwrap()) {
                    Ok(mut conf) => {
                        match kg.restart(command.child_name.as_ref().unwrap(), &mut conf) {
                            Ok(_) => (),
                            Err(e) => println!("{}", e),
                        }
                    }
                    Err(e) => {
                        println!("{:?}", e);
                    }
                }
            }
            _ => (),
        }
    }
}

//get client TCP stream and send to channel
fn handle_client(mut stream: TcpStream, sd: Sender<String>) -> Result<()> {
    let mut buf = vec![];
    stream.read_to_end(&mut buf)?;

    let received_comm = String::from_utf8(buf).unwrap();
    sd.send(received_comm).unwrap();
    //:= TODO: maybe have input legal check
    Ok(())
}

//start a listener for client commands
//keep taking care children
pub fn start_deamon(kg: Kindergarten) -> Result<(thread::JoinHandle<()>, thread::JoinHandle<()>)> {
    //channel used to communicate from listener and day care
    let (sender, receiver) = channel::<String>();

    //start TCP listener to receive client commands
    let listener = TcpListener::bind(format!("{}:{}", "127.0.0.1", 33889))?;
    let handler_of_client = thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let thread_sender = sender.clone();
                    if let Err(e) = handle_client(stream, thread_sender) {
                        println!("{}", e);
                    };
                }
                Err(e) => println!("{}", e),
            }
        }
    });

    //let kg = Kindergarten::new();
    let handler_of_day_care = thread::spawn(move || {
        day_care(kg, receiver);
    });

    Ok((handler_of_client, handler_of_day_care))
}

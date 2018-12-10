use super::communication::*;
use super::Config;

use core::time::Duration;
use std::collections::HashMap;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::{Error as ioError, ErrorKind, Read, Result};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use yaml_rust::{ScanError, Yaml, YamlEmitter, YamlLoader};

/*/tmp/server.yml
loadpath:
  - /tmp/client/
 */

#[derive(Debug)]
struct ServerConfig {
    //path for all children's configs
    load_path: String,
}

impl ServerConfig {
    fn load(filename: &str) -> Result<Self> {
        let mut contents = File::open(filename)?;
        let mut string_result = String::new();

        contents.read_to_string(&mut string_result);
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
}

#[derive(Debug)]
pub struct Kindergarten {
    id_list: HashMap<u32, Config>,
    name_list: HashMap<String, u32>,
}

impl Kindergarten {
    pub fn new() -> Self {
        Kindergarten {
            id_list: HashMap::new(),
            name_list: HashMap::new(),
        }
    }

    pub fn register_id(&mut self, id: u32, conf: Config) {
        self.id_list.insert(id, conf);
    }

    pub fn register_name(&mut self, name: String, id: u32) {
        self.name_list.insert(name, id);
    }

    pub fn update(&mut self, id: u32, name: String, conf: Config) {
        self.register_id(id, conf);
        self.register_name(name, id);
    }
}

//start a child processing
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

    child
}

//for deamon to start new child
//receive file path, make it to config, then start it
//return id, config for deamon to update kindergarten
pub fn start_new_child_with_file(filepath: &str) -> Result<(u32, Config)> {
    if let Ok(mut conf) = Config::read_from_yaml_file(filepath) {
        if let Ok(child) = start_new_child(&mut conf) {
            return Ok((conf.child_id.unwrap(), conf));
        }
    }

    Err(ioError::new(
        ErrorKind::Other,
        format!("Cannot start this, file is {}", filepath),
    ))
}

//Receive server config and start a new server
//new server including:
//1. a way receive command from client //move to start_deamon
//2. first start will start all children in config path
//3. then keep listening commands and can restart each of them //move to start deamon
pub fn start_new_server() -> Result<Kindergarten> {
    //Read server's config file
    let mut server_conf = ServerConfig::load("/tmp/server.yml")?;

    //create new kindergarten
    let mut kindergarten = Kindergarten::new();

    //run all config already in load path
    for entry in fs::read_dir(server_conf.load_path)? {
        if let Ok(entry) = entry {
            if let Some(extension) = entry.path().extension() {
                if extension == "yml" {
                    if let Ok(mut child_config) =
                        Config::read_from_yaml_file(entry.path().to_str().unwrap())
                    {
                        //start processing
                        if let Err(_) = start_new_child(&mut child_config) {
                            continue;
                        }

                        //registe id
                        let id = child_config.child_id.unwrap();
                        kindergarten.register_id(id, child_config);

                        //regist name
                        let filename = entry
                            .file_name()
                            .to_str()
                            .unwrap()
                            .split('.')
                            .collect::<Vec<&str>>()[0]
                            .to_string();
                        kindergarten.register_name(filename, id);
                    }
                }
            }
        }
    }

    Ok(kindergarten)
}

//check all children are fine or not
//if not fine, try to restart them
//need channel input to update kindergarten
fn day_care(kg: Kindergarten, rec: Receiver<String>) {
    loop {
        //thread::sleep(Duration::from_secs(1));
        //let (command, argvs) = rec.recv().unwrap();
        println!("{:?}", rec.recv().unwrap());
    }
}

//get client TCP stream and send to channel
fn handle_client(mut stream: TcpStream, sd: Sender<String>) {
    let mut buf = vec![];
    stream.read_to_end(&mut buf);

    //println!("{}", String::from_utf8(buf).unwrap());
    let received_string = String::from_utf8(buf).unwrap();
    sd.send(received_string);
    //:= TODO: maybe have input legal check
}

//start a listener for client commands
//keep taking care children
pub fn start_deamon(kg: Kindergarten) -> Result<(thread::JoinHandle<()>, thread::JoinHandle<()>)> {
    //channel used to communicate from listener and day care
    let (sender, receiver) = channel::<String>();

    //start TCP listener to receive client commands
    let listener = TcpListener::bind(format!("{}:{}", "127.0.0.1", 33889))?;
    let handler_of_client = thread::spawn(move || {
        println!("inside");
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let thread_sender = sender.clone();
                    handle_client(stream, thread_sender);
                    println!("new client!");
                }
                Err(e) => { /* connection failed */ }
            }
        }
    });

    let kg = Kindergarten::new();
    let handler_of_day_care = thread::spawn(move || {
        println!("inside");
        day_care(kg, receiver);
    });

    Ok((handler_of_client, handler_of_day_care))
}

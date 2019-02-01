use super::client;
use super::kindergarten::*;
use super::Config;

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

//start a child processing, and give child_handle
//side effection: config.child_id be updated
//:= TODO: stdout and stderr should have ability to append file
//:= TODO: start and restart should have more detail
pub fn start_new_child(config: &mut Config) -> Result<Child> {
    let (com, args) = config.split_args();

    let mut command = Command::new(&com);

    match args {
        Some(arg) => {
            command.args(arg.split(' ').collect::<Vec<&str>>());
        }
        _ => (),
    };

    //setting stdout and stderr file path
    match &config.stdout {
        Some(out) => {
            command.stdout(File::create(out)?);
            ()
        }
        None => (),
    }

    match &config.stderr {
        Some(err) => {
            command.stdout(File::create(err)?);
            ()
        }
        None => (),
    }

    //run command and give child handle
    let child = command.spawn();

    match child {
        Ok(ref c) => {
            config.child_id = Some(c.id());
            return child;
        }
        _ => {
            return Err(ioError::new(
                ErrorKind::Other,
                format!("Cannot start command {:?}", command),
            ));
        }
    };
}

//Receive server config and start a new server
//new server including:
//1. a way receive command from client //move to start_deamon
//2. first start will start all children in config path
//3. then keep listening commands and can restart each of them //move to start deamon
//:= TODO: child is not server application, check logic is fine
pub fn start_new_server(config_path: &str) -> Result<Kindergarten> {
    //Read server's config file
    let server_conf = if config_path == "" {
        ServerConfig::load("/tmp/server.yml")?
    } else {
        ServerConfig::load(config_path)?
    };

    //create new kindergarten
    let mut kindergarten = Kindergarten::new();

    //store server config location
    kindergarten.server_config_path = config_path.to_string();

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
//:= TODO: illegal command should return more details
fn day_care(mut kg: Kindergarten, rec: Receiver<String>) {
    loop {
        //println!("{:#?}", kg);
        let data = rec.recv().unwrap();
        let command = if let Ok(com) =
            client::Command::new_from_str(data.as_str().split(' ').collect::<Vec<&str>>())
        {
            com
        } else {
            continue;
        };

        match command.op {
            client::Ops::Restart => {
                let server_conf = if kg.server_config_path == "" {
                    ServerConfig::load("/tmp/server.yml")
                } else {
                    ServerConfig::load(&kg.server_config_path)
                };

                match server_conf {
                    Ok(s_conf) => {
                        match s_conf.find_config_by_name(command.child_name.as_ref().unwrap()) {
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
                    Err(e) => println!("Cannot re-load server's config file, {}", e),
                }
            }

            // warm start a new child after its config yaml file put in loadpath
            client::Ops::Start => {
                let server_conf = if kg.server_config_path == "" {
                    ServerConfig::load("/tmp/server.yml")
                } else {
                    ServerConfig::load(&kg.server_config_path)
                };

                match server_conf {
                    Ok(s_conf) => {
                        match s_conf.find_config_by_name(command.child_name.as_ref().unwrap()) {
                            Ok(mut conf) => match start_new_child(&mut conf) {
                                Ok(child_handle) => {
                                    let id = conf.child_id.unwrap();
                                    kg.register_id(id, child_handle, conf);
                                    kg.register_name(&command.child_name.unwrap(), id);
                                }
                                Err(e) => println!("{:?}", e),
                            },
                            Err(e) => {
                                println!("{:?}", e);
                            }
                        }
                    }
                    Err(e) => println!("Cannot re-load server's config file, {}", e),
                }
            }

            client::Ops::Stop => match kg.stop(command.child_name.as_ref().unwrap()) {
                Ok(_) => (),
                Err(e) => println!("{}", e),
            },
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

    Ok(())
}

//start a listener for client commands
//keep taking care children
pub fn start_deamon(kg: Kindergarten) -> Result<(thread::JoinHandle<()>, thread::JoinHandle<()>)> {
    //channel used to communicate from listener and day care
    let (sender, receiver) = channel::<String>();

    //start TCP listener to receive client commands
    let listener = TcpListener::bind(format!("{}:{}", "0.0.0.0", 33889))?;
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

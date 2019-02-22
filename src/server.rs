use super::client;
use super::kindergarten::*;
use super::{Config, OutputMode};

use std::error::Error;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{Error as ioError, ErrorKind, Read, Result, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command};
use std::sync::mpsc::Sender;
use std::thread;
use yaml_rust::YamlLoader;

use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct ServerConfig {
    //path for all children's configs
    load_paths: Vec<String>,
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
        let mut result: Self = ServerConfig { load_paths: vec![] };

        match temp {
            Ok(docs) => {
                let doc = &docs[0];
                let paths = match doc["loadpaths"].as_vec() {
                    Some(v) => v
                        .iter()
                        .map(|x| x.clone().into_string().unwrap())
                        .collect::<Vec<String>>(),
                    None => return Ok(result),
                };

                result = ServerConfig { load_paths: paths }
            }
            Err(e) => return Err(ioError::new(ErrorKind::Other, e.description().to_string())),
        }

        Ok(result)
    }

    //return vec of (filename, path)
    fn all_ymls_in_load_path(&self) -> Result<Vec<(String, String)>> {
        let mut result: Vec<(String, String)> = vec![];
        for path in self.load_paths.clone() {
            for entry in fs::read_dir(path)? {
                if let Ok(entry) = entry {
                    if let Some(extension) = entry.path().extension() {
                        if extension == "yml" || extension == "yaml" {
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
        }
        Ok(result)
    }

    //return whole path of file which match filename
    fn find_config_by_name(&self, filename: &String) -> Result<Config> {
        for path in self.load_paths.clone() {
            for entry in fs::read_dir(path)? {
                if let Ok(entry) = entry {
                    if let Some(extension) = entry.path().extension() {
                        if extension == "yml" || extension == "yaml" {
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
        }

        Err(ioError::new(
            ErrorKind::NotFound,
            format!("Cannot found '{}' file in load path", filename),
        ))
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

    //setting stdout and stderr file path
    match &config.stdout {
        Some(out) => {
            match out.mode {
                OutputMode::Create => command.stdout(File::create(out.path.clone())?),
                OutputMode::Append => command.stdout(
                    OpenOptions::new()
                        .append(true)
                        .create(true)
                        .open(out.path.clone())?,
                ),
            };
            ()
        }
        None => (),
    }

    match &config.stderr {
        Some(err) => {
            match err.mode {
                OutputMode::Create => command.stderr(File::create(err.path.clone())?),
                OutputMode::Append => command.stderr(
                    OpenOptions::new()
                        .append(true)
                        .create(true)
                        .open(err.path.clone())?,
                ),
            };
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
pub fn start_new_server(config_path: &str, arg: &str) -> Result<Kindergarten> {
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

    //if arg == -q, don't run initial
    if arg != "-q" {
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
    }

    Ok(kindergarten)
}

//start a listener for client commands
//keep taking care children
pub fn start_deamon(kg: Kindergarten, sd: Sender<(String, String)>) -> Result<()> {
    let safe_kg = Arc::new(Mutex::new(kg));

    //start TCP listener to receive client commands
    let listener = TcpListener::bind(format!("{}:{}", "0.0.0.0", 33889))?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let this_kg = Arc::clone(&safe_kg);
                let sd_ = Sender::clone(&sd);
                let _ = thread::spawn(move || {
                    //run handle_client and catch error if has
                    if let Err(e) = handle_client(stream, this_kg) {
                        //hard check if it is suicide operation, because I only care this message so far.
                        //this operation isn't in handle_client because it has make sure return to client..
                        //..first
                        let (first, second) = e.description().split_at(12);
                        if first == "I am dying. " {
                            println!("{}", second.to_string());
                            //tell main thread,
                            sd_.send((first.to_string(), second.to_string())).unwrap();
                        } else {
                            //if just normal error
                            println!("{}", e.description());
                        }
                    };
                });
            }

            Err(e) => println!("{}", e),
        }
    }

    Ok(())
}

//get client TCP stream and send to channel
fn handle_client(mut stream: TcpStream, kg: Arc<Mutex<Kindergarten>>) -> Result<()> {
    let mut buf = [0; 100];
    stream.read(&mut buf)?;

    let mut buf_vec = buf.to_vec();
    buf_vec.retain(|&x| x != 0);

    let received_comm = String::from_utf8(buf_vec).unwrap();

    match day_care(kg, received_comm) {
        Ok(resp) => stream.write_all(format!("server response: \n{}", resp).as_bytes()),
        Err(e) => {
            stream.write_all(format!("server response error: \n{}", e.description()).as_bytes())?;
            Err(e)
        }
    }
}

//check all children are fine or not
//if not fine, try to restart them
//need channel input to update kindergarten
fn day_care(kig: Arc<Mutex<Kindergarten>>, data: String) -> Result<String> {
    //:= TODO: need recover poisoned mutex
    let mut kg = kig.lock().unwrap();

    //run check around here, clean all stopped children
    //check operation has its own check_around too, check_around here..
    //..for other operations.
    kg.check_around()?;

    let command = client::Command::new_from_str(data.as_str().split(' ').collect::<Vec<&str>>())?;

    match command.op {
        client::Ops::Restart => {
            let server_conf = if kg.server_config_path == "" {
                ServerConfig::load("/tmp/server.yml")?
            } else {
                ServerConfig::load(&kg.server_config_path)?
            };

            let mut conf = server_conf.find_config_by_name(command.child_name.as_ref().unwrap())?;

            match kg.restart(command.child_name.as_ref().unwrap(), &mut conf) {
                Ok(_) => Ok(format!(
                    "restart {} success",
                    command.child_name.as_ref().unwrap()
                )),
                Err(e) => Err(e),
            }
        }

        // hot start a new child after its config yaml file put in loadpath
        client::Ops::Start => {
            if let Some(_) = kg.has_child(command.child_name.as_ref().unwrap()) {
                return Err(ioError::new(
                    ErrorKind::Other,
                    format!(
                        "Cannot start this child {}, it already exsits",
                        command.child_name.unwrap()
                    ),
                ));
            }

            let server_conf = if kg.server_config_path == "" {
                ServerConfig::load("/tmp/server.yml")?
            } else {
                ServerConfig::load(&kg.server_config_path)?
            };

            let mut conf = server_conf.find_config_by_name(command.child_name.as_ref().unwrap())?;
            match start_new_child(&mut conf) {
                Ok(child_handle) => {
                    let id = conf.child_id.unwrap();
                    kg.register_id(id, child_handle, conf);
                    kg.register_name(command.child_name.as_ref().unwrap(), id);
                    Ok(format!(
                        "start {} success",
                        command.child_name.as_ref().unwrap()
                    ))
                }
                Err(e) => Err(e),
            }
        }

        client::Ops::Stop => match kg.stop(command.child_name.as_ref().unwrap()) {
            Ok(_) => {
                return Ok(format!(
                    "stop {} success",
                    command.child_name.as_ref().unwrap()
                ));
            }
            Err(e) => Err(e),
        },

        //try start will force start child:
        //if it is running, call restart.
        //if it has stopped for some reason, start it
        client::Ops::TryStart => {
            let mut resp = String::new();

            //check if it is running, stop it or not.
            if let Some(_) = kg.has_child(command.child_name.as_ref().unwrap()) {
                resp.push_str("this child already start, stop it first.\n");
                match kg.stop(command.child_name.as_ref().unwrap()) {
                    Ok(_) => {
                        resp.push_str(&format!(
                            "stop {} success, start it again.\n",
                            command.child_name.as_ref().unwrap()
                        ));
                    }
                    Err(e) => {
                        return Err(ioError::new(
                            ErrorKind::InvalidData,
                            format!("stop failed, error: {}\n", e.description()),
                        ));
                    }
                }
            }

            //start it again, same as Ops::Start branch
            let server_conf = if kg.server_config_path == "" {
                ServerConfig::load("/tmp/server.yml")?
            } else {
                ServerConfig::load(&kg.server_config_path)?
            };

            let mut conf = server_conf.find_config_by_name(command.child_name.as_ref().unwrap())?;
            match start_new_child(&mut conf) {
                Ok(child_handle) => {
                    let id = conf.child_id.unwrap();
                    kg.register_id(id, child_handle, conf);
                    kg.register_name(command.child_name.as_ref().unwrap(), id);
                    resp.push_str(&format!(
                        "start {} success",
                        command.child_name.as_ref().unwrap()
                    ));
                    Ok(resp)
                }
                Err(e) => Err(e),
            }
        }

        // kill supervisor itself
        client::Ops::Kill => {
            let mut last_will = String::new();
            // step1: stop all
            if let Err(e) = kg.stop(&"all".to_string()) {
                last_will.push_str(&format!("there is error when stop all {}", e.description()));
            }

            // step2: return special err outside, let deamon know and stop
            Err(ioError::new(
                ErrorKind::Other,
                format!("I am dying. last error: {}", last_will),
            ))
        }

        client::Ops::Check => kg.check_status(command.child_name.as_ref().unwrap()),
    }
}

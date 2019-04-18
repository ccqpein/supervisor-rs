use super::client;
use super::kindergarten::*;
use super::logger;
use super::timer::*;
use super::{Config, OutputMode};

use std::collections::HashSet;
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
    mode: String,
    startup_list: Option<Vec<String>>,
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
        let mut result: Self = ServerConfig {
            load_paths: vec![],

            //quiet mode is default value now
            mode: "quiet".to_string(),
            startup_list: None,
        };

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
                result.load_paths = paths;

                let mode = match doc["mode"].as_str() {
                    Some(v) => v.to_string(),
                    None => return Ok(result),
                };
                result.mode = mode;

                let startup_children = match doc["startup"].as_vec() {
                    Some(v) => v
                        .iter()
                        .map(|x| x.clone().into_string().unwrap())
                        .collect::<Vec<String>>(),
                    None => return Ok(result),
                };
                result.startup_list = Some(startup_children);
            }
            Err(e) => return Err(ioError::new(ErrorKind::Other, e.description().to_string())),
        }
        Ok(result)
    }

    //when mode == "half"
    //it should return all children details those in loadpaths also...
    //in startup field of server config.
    fn half_mode(&self) -> Result<Vec<(String, String)>> {
        let children_set = match &self.startup_list {
            Some(startups) => startups.iter().collect::<HashSet<&String>>(),
            None => return Err(ioError::new(ErrorKind::NotFound, "startup list not found")),
        };

        let all_children = self.all_ymls_in_load_path()?;

        let result = all_children
            .into_iter()
            .filter(|x| children_set.contains(&x.1))
            .collect::<Vec<(String, String)>>();

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

fn child_name_legal_check(s: &str) -> core::result::Result<(), String> {
    if s == "all" || s == "on" {
        return Err(format!(
            r#""{}" is reserved keywords, cannot run child named "{}""#,
            s, s
        ));
    } else if client::Ops::is_op(s) {
        //check if child name is keyword or not.
        return Err(format!(
            "this child name {} is one of keyword, cannot be child name",
            s
        ));
    }

    Ok(())
}

//Receive server config and start a new server
//new server including:
//1. a way receive command from client //move to start_deamon
//2. first start will start all children in config path
//3. then keep listening commands and can restart each of them //move to start deamon
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

    //make startup children vec
    let startup_children = match server_conf.mode.as_ref() {
        "full" => server_conf.all_ymls_in_load_path()?,
        "half" => server_conf.half_mode()?,
        "quiet" | _ => vec![],
    };

    //print log
    if startup_children.len() != 0 {
        println!(
            "{}",
            logger::timelog(&format!(
                "these children will start with server startup: {:?}",
                startup_children
                    .iter()
                    .map(|x| x.1.clone())
                    .collect::<Vec<String>>()
            ))
        );
    }

    //start children with server
    for conf in startup_children {
        //legal check child name
        //because client already check when it makes command...
        //..., however, server un-quiet mode won't through client command check.
        //so we need check again here.
        if let Err(e) = child_name_legal_check(&conf.0) {
            println!("{}", logger::timelog(&e));
            continue;
        };

        let mut child_config = Config::read_from_yaml_file(&conf.1)?;

        let child_handle = start_new_child(&mut child_config)?;

        println!("{}", logger::timelog(&format!("start {} success", conf.0)));

        if child_config.is_repeat() {
            println!(
                "{}",
                logger::timelog(&format!(
                    "find child {} have repeat status, not support repeat in un-quiet mode",
                    &conf.0
                ))
            )
        };

        //registe id
        let id = child_config.child_id.unwrap();
        kindergarten.register_id(id, child_handle, child_config);
        //regist name
        kindergarten.register_name(&conf.0, id);
    }

    Ok(kindergarten)
}

//start a listener for client commands
//keep taking care children
pub fn start_deamon(safe_kg: Arc<Mutex<Kindergarten>>, sd: Sender<(String, String)>) -> Result<()> {
    //start TCP listener to receive client commands
    let listener = TcpListener::bind(format!("{}:{}", "0.0.0.0", 33889))?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let this_kg = Arc::clone(&safe_kg);
                let sd_ = Sender::clone(&sd);
                let _ = thread::spawn(move || {
                    //run handle_client and catch error if has
                    match handle_client(stream, this_kg) {
                        Err(e) => {
                            //hard check if it is suicide operation, because I only care this message so far.
                            //this operation isn't in handle_client because it has make sure return to client..
                            //..first
                            let (first, second) = e.description().split_at(12);
                            if first == "I am dying. " {
                                print!("{}", logger::timelog(second));
                                //tell main thread,
                                sd_.send((first.to_string(), second.to_string())).unwrap();
                            } else {
                                //if just normal error
                                println!("{}", logger::timelog(e.description()));
                            }
                        }
                        Ok(des) => println!("{}", logger::timelog(&des)),
                    }
                });
            }

            Err(e) => println!("{}", logger::timelog(e.description())),
        }
    }

    Ok(())
}

//get client TCP stream and send to channel
fn handle_client(mut stream: TcpStream, kig: Arc<Mutex<Kindergarten>>) -> Result<String> {
    let mut buf = [0; 100];
    stream.read(&mut buf)?;

    let mut buf_vec = buf.to_vec();
    buf_vec.retain(|&x| x != 0);

    let received_comm = String::from_utf8(buf_vec).unwrap();

    match day_care(kig, received_comm) {
        Ok(resp) => {
            stream.write_all(format!("server response: \n{}", resp).as_bytes())?;
            Ok(resp)
        }
        Err(e) => {
            stream.write_all(format!("server response error: \n{}", e.description()).as_bytes())?;
            Err(e)
        }
    }
}

//check all children are fine or not
//if not fine, try to restart them
//need channel input to update kindergarten
pub fn day_care(kig: Arc<Mutex<Kindergarten>>, data: String) -> Result<String> {
    let mut kg = kig.lock().unwrap();

    //run check around here, clean all stopped children
    //check operation has its own check_around too, check_around here..
    //..for other operations.
    kg.check_around()?;

    let command = client::Command::new_from_str(data.as_str().split(' ').collect::<Vec<&str>>())?;

    match command.op {
        client::Ops::Restart => {
            let name = command.child_name.as_ref().unwrap();
            //check name
            if let Err(e) = child_name_legal_check(name) {
                return Err(ioError::new(ErrorKind::InvalidInput, e));
            }

            let server_conf = if kg.server_config_path == "" {
                ServerConfig::load("/tmp/server.yml")?
            } else {
                ServerConfig::load(&kg.server_config_path)?
            };

            let mut conf = server_conf.find_config_by_name(name)?;

            match kg.restart(name, &mut conf) {
                Ok(_) => {
                    //repeat here
                    let repeat_meg = if conf.is_repeat() {
                        repeat(conf, Arc::clone(&kig), name.clone())
                    } else {
                        String::new()
                    };

                    Ok(format!(
                        "restart {} success{}\n",
                        command.child_name.as_ref().unwrap(),
                        repeat_meg,
                    ))
                }
                Err(e) => Err(e),
            }
        }

        // hot start a new child after its config yaml file put in loadpath
        client::Ops::Start => {
            let name = command.child_name.as_ref().unwrap();
            if let Err(e) = child_name_legal_check(name) {
                return Err(ioError::new(ErrorKind::InvalidInput, e));
            }

            if let Some(_) = kg.has_child(name) {
                return Err(ioError::new(
                    ErrorKind::Other,
                    format!(
                        "Cannot start this child {}, it already exsits\n",
                        command.child_name.unwrap()
                    ),
                ));
            }

            let server_conf = if kg.server_config_path == "" {
                ServerConfig::load("/tmp/server.yml")?
            } else {
                ServerConfig::load(&kg.server_config_path)?
            };

            let mut conf = server_conf.find_config_by_name(&name)?;

            match start_new_child(&mut conf) {
                Ok(child_handle) => {
                    let id = conf.child_id.unwrap();
                    kg.register_id(id, child_handle, conf.clone());
                    kg.register_name(&name, id);

                    //repeat here
                    let repeat_meg = if conf.is_repeat() {
                        repeat(conf, Arc::clone(&kig), name.clone())
                    } else {
                        String::new()
                    };

                    Ok(format!("start {} success{}\n", name.clone(), repeat_meg))
                }
                Err(e) => Err(e),
            }
        }

        client::Ops::Stop => match kg.stop(command.child_name.as_ref().unwrap()) {
            Ok(_) => {
                return Ok(format!(
                    "stop {} success\n",
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

            let name = command.child_name.as_ref().unwrap();
            if let Err(e) = child_name_legal_check(name) {
                return Err(ioError::new(ErrorKind::InvalidInput, e));
            };

            //check if it is running, stop it or not.
            if let Some(_) = kg.has_child(name) {
                resp.push_str("this child already start, stop it first.\n");
                match kg.stop(name) {
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

            let mut conf = server_conf.find_config_by_name(name)?;

            match start_new_child(&mut conf) {
                Ok(child_handle) => {
                    let id = conf.child_id.unwrap();
                    kg.register_id(id, child_handle, conf.clone());
                    kg.register_name(command.child_name.as_ref().unwrap(), id);

                    //repeat here
                    let repeat_meg = if conf.is_repeat() {
                        repeat(conf, Arc::clone(&kig), name.clone())
                    } else {
                        String::new()
                    };

                    resp.push_str(&format!(
                        "start {} success{}\n",
                        command.child_name.as_ref().unwrap(),
                        repeat_meg,
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
                last_will.push_str(&format!(
                    "there is error when stop all {}\n",
                    e.description()
                ));
            }

            // step2: return special err outside, let deamon know and stop
            Err(ioError::new(
                ErrorKind::Other,
                format!("I am dying. last error: \n{}\n", last_will),
            ))
        }

        client::Ops::Check => kg.check_status(command.child_name.as_ref().unwrap()),

        _ => {
            return Err(ioError::new(
                ErrorKind::InvalidInput,
                logger::timelog("not support"),
            ))
        }
    }
}

//receive child config, KG, and filename of child config, repeat function
fn repeat(conf: Config, kig: Arc<Mutex<Kindergarten>>, name: String) -> String {
    //clone locked val to timer
    let timer_lock_val = Arc::clone(&kig);
    let next_time = conf.to_duration().unwrap();
    let comm = conf.repeat_command().unwrap().clone();

    //give a timer
    thread::spawn(move || {
        Timer::new_from_conf(name, conf)
            .unwrap()
            .run(timer_lock_val) //:= TODO: need command parser
    });
    format!(", and it will {} in {:?}", comm, next_time)
}

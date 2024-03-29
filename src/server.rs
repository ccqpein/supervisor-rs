use super::child::{child_output::OutputMode, Config};
use super::client;
use super::keys_handler::*;
use super::kindergarten::*;
use super::logger;
use super::timer::*;

use chrono::prelude::*;
use openssl::rsa::*;
use std::collections::HashSet;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{Error as ioError, ErrorKind, Read, Result, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command};
use std::sync::mpsc::Sender;
use std::thread;
use yaml_rust::YamlLoader;

use std::sync::{Arc, Mutex};

/// Server config
#[derive(Debug)]
struct ServerConfig {
    /// paths for all children's configs
    load_paths: Vec<String>,

    /// startup mode
    /// + half
    /// + full
    /// + quiet (default)
    mode: String,

    /// The list of children want to start up with server
    startup_list: Option<Vec<String>>,

    /// encrypt mode
    encrypt_mode: String,

    /// client public keys location
    keys_path: Option<Vec<String>>,

    /// Listener address
    /// default is 0.0.0.0, ipv4
    listener_addr: String,

    /// Enable ipv6 or not, only used when listener_addr
    /// isn't given in yaml file. If listener_addr appears in yaml,
    /// this field doesn't matter.
    /// If listener_addr isn't given and this field is true,
    /// listener_addr will become ::
    ipv6: bool,
}

impl ServerConfig {
    /// load server config
    fn load(filepath: &str) -> Result<Self> {
        let mut contents = File::open(filepath)?;
        let mut string_result = String::new();

        contents.read_to_string(&mut string_result)?;
        Self::read_from_str(string_result.as_str())
    }

    /// read content of config file then making config
    fn read_from_str(input: &str) -> Result<Self> {
        let temp = YamlLoader::load_from_str(input);
        let mut result: Self = ServerConfig {
            load_paths: vec![],

            // quiet mode is default value now
            mode: "quiet".to_string(),
            startup_list: None,

            encrypt_mode: "off".to_string(),
            keys_path: None,

            listener_addr: "0.0.0.0".to_string(),
            ipv6: false,
        };

        match temp {
            Ok(docs) => {
                let doc = &docs[0];

                // load path parse
                let paths = match doc["loadpaths"].as_vec() {
                    Some(v) => v
                        .iter()
                        .map(|x| x.clone().into_string().unwrap())
                        .collect::<Vec<String>>(),
                    None => vec![],
                };
                result.load_paths = paths;

                // mode parse
                let mode = match doc["mode"].as_str() {
                    Some(v) => v.to_string(),
                    None => "quiet".to_string(),
                };
                result.mode = mode;

                // startup parse
                let startup_children = match doc["startup"].as_vec() {
                    Some(v) => Some(
                        v.iter()
                            .map(|x| x.clone().into_string().unwrap())
                            .collect::<Vec<String>>(),
                    ),
                    None => None,
                };
                result.startup_list = startup_children;

                // encrypt parse
                let encrypt = match doc["encrypt"].as_str() {
                    Some(v) => v.to_string(),
                    None => "off".to_string(),
                };
                result.encrypt_mode = encrypt;

                // keys path parse
                let keys_paths = match doc["pub_keys_path"].as_vec() {
                    Some(v) => Some(
                        v.iter()
                            .map(|x| x.clone().into_string().unwrap())
                            .collect::<Vec<String>>(),
                    ),
                    None => None,
                };
                result.keys_path = keys_paths;

                // ipv6
                let p6 = match doc["ipv6"].as_bool() {
                    Some(b) => b,
                    None => false,
                };
                result.ipv6 = p6;

                // listener part
                let listener_addr = match doc["listener_addr"].as_str() {
                    Some(addr) => addr.to_string(),
                    None => {
                        if !p6 {
                            "0.0.0.0".to_string()
                        } else {
                            "::".to_string()
                        }
                    }
                };
                result.listener_addr = listener_addr;
            }
            Err(e) => return Err(ioError::new(ErrorKind::Other, e)),
        }
        Ok(result)
    }

    /// when mode == "half"
    /// it should return all children details those in loadpaths also
    /// in startup field of server config.
    fn half_mode(&self) -> Result<Vec<(String, String)>> {
        let children_set = match &self.startup_list {
            Some(startups) => startups.iter().collect::<HashSet<&String>>(),
            None => return Err(ioError::new(ErrorKind::NotFound, "startup list not found")),
        };

        let all_children = self.all_ymls_in_load_path()?;

        let result = all_children
            .into_iter()
            .filter(|x| children_set.contains(&x.0))
            .collect::<Vec<(String, String)>>();

        Ok(result)
    }

    /// Return all children configs. Vec of (filename, path)
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

    /// Return config which match filename
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
                                return Config::read_from_yaml_file(entry.path());
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

    /// Return key's path
    fn find_pubkey_by_name(&self, filename: &String) -> Result<String> {
        if let Some(paths) = &self.keys_path {
            for path in paths {
                for entry in fs::read_dir(path)? {
                    if let Ok(entry) = entry {
                        if let Some(extension) = entry.path().extension() {
                            if extension == "pem" {
                                if entry
                                    .file_name()
                                    .to_str()
                                    .unwrap()
                                    .split('.')
                                    .collect::<Vec<&str>>()[0]
                                    .to_string()
                                    == *filename
                                {
                                    return Ok(entry
                                        .path()
                                        .into_os_string()
                                        .into_string()
                                        .unwrap());
                                }
                            }
                        }
                    }
                }
            }
        }

        Err(ioError::new(
            ErrorKind::NotFound,
            format!("Cannot found '{}' file in keys path", filename),
        ))
    }

    /// recursive check if config have dead loop prehook call
    fn recursive_check(
        &self,
        start: &Config,
        set: &mut HashSet<String>,
        chain: &mut Vec<(String, String)>,
    ) -> bool {
        if let Some(hook) = start.get_hook_detail(&String::from("prehook")) {
            // if hashset already has this child in set, return wrong
            if set.contains(&hook[1]) {
                return false;
            }

            // Get prehook child's config
            if let Ok(next_config) = self.find_config_by_name(&hook[1]) {
                // insert in set
                set.insert(hook[1].clone());
                // push this one in call_chain
                chain.push((hook[0].clone(), hook[1].clone()));
                // keep checking
                return self.recursive_check(&next_config, set, chain);
            }
            return false; // if hook child not exsit
        }
        true // test fine
    }

    /// Get config prehook details, find them one by one make sure no circle inside
    fn pre_hook_check(
        &self,
        name: &String,
        start: &Config,
    ) -> Result<(bool, Vec<(String, String)>)> {
        let mut call_set = HashSet::new();
        let mut call_chain = vec![]; // chain including (command, name)

        // put this
        call_set.insert(name.clone());

        if self.recursive_check(start, &mut call_set, &mut call_chain) {
            Ok((true, call_chain))
        } else {
            Ok((false, call_chain))
        }
    }

    /// Generate call chain used by KG to handle prehooks
    fn call_chain_combine(
        &self,
        call_chain: Vec<(String, String)>,
    ) -> Result<Vec<(String, String, Config)>> {
        let all_child = self.all_ymls_in_load_path()?;
        let mut result = vec![];

        for (comm, name) in call_chain {
            if let Some(conf_path) = all_child.iter().find(|x| x.0 == name) {
                if let Ok(conf) = Config::read_from_yaml_file((&conf_path.1).into()) {
                    result.push((comm, name, conf));
                }
            }
        }
        Ok(result)
    }
}

/// Start a child processing, and give child_handle
/// Side effection: config.child_id be updated
pub fn start_new_child(config: &mut Config) -> Result<Child> {
    let (com, args) = config.split_args();

    let mut command = Command::new(&com);

    command.current_dir(config.location_path.clone());

    match args {
        Some(arg) => {
            command.args(arg.split(' ').collect::<Vec<&str>>());
        }
        _ => (),
    };

    // setting stdout and stderr file path
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
            config.start_time = Some(Local::now());
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

/// Check if child name is legal or not
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

/// Receive server config and start a new server
/// New server including:
/// 1. a way receive command from client //move to start_deamon
/// 2. first start will start all children in config path
/// 3. then keep listening commands and can restart each of them //move to start deamon
pub fn start_new_server(config_path: &str) -> Result<Kindergarten> {
    // Read server's config file
    let server_conf = if config_path == "" {
        ServerConfig::load("/tmp/server.yml")?
    } else {
        ServerConfig::load(config_path)?
    };

    // create new kindergarten
    let mut kindergarten = Kindergarten::new();

    // store server config location
    // after this, server_config_path should never changed
    kindergarten.server_config_path = config_path.to_string();

    // kindergarden make encrypt on
    if server_conf.encrypt_mode == "on" {
        kindergarten.encrypt_mode = true;
    }

    // make startup children vec
    let startup_children = match server_conf.mode.as_ref() {
        "full" => server_conf.all_ymls_in_load_path()?,
        "half" => server_conf.half_mode()?,
        "quiet" | _ => vec![],
    };

    // print log
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

    // start children with server
    for conf in startup_children {
        // legal check child name
        // because client already check when it makes command...
        // ..., however, server un-quiet mode won't through client command check.
        // so we need check again here.
        if let Err(e) = child_name_legal_check(&conf.0) {
            println!("{}", logger::timelog(&e));
            continue;
        };

        let mut child_config = Config::read_from_yaml_file((&conf.1).into())?;

        let child_handle = start_new_child(&mut child_config)?;

        println!("{}", logger::timelog(&format!("start {} success", conf.0)));

        // because repeat function need kindergarden be created.
        if child_config.is_repeat() {
            println!(
                "{}",
                logger::timelog(&format!(
                    "find child {} has repeat status, not support repeat in un-quiet mode during server startup",
                    &conf.0
                ))
            )
        };

        // because repeat function need kindergarden be created.
        if child_config.has_hook() {
            println!(
                "{}",
                logger::timelog(&format!(
                    "find child {} has hook(s), not support prehook in un-quiet mode during server startup",
                    &conf.0
                ))
            )
        };

        // registe id
        let id = child_config.child_id.unwrap();
        kindergarten.register_id(id, child_handle, child_config);
        // regist name
        kindergarten.register_name(&conf.0, id);
    }

    Ok(kindergarten)
}

/// start a listener for client commands
/// keep taking care children
pub fn start_deamon(safe_kg: Arc<Mutex<Kindergarten>>, sd: Sender<(String, String)>) -> Result<()> {
    // re-read server config
    let server_conf = if safe_kg.lock().unwrap().server_config_path == "" {
        ServerConfig::load("/tmp/server.yml")?
    } else {
        ServerConfig::load(&safe_kg.lock().unwrap().server_config_path)?
    };

    // start TCP listener to receive client commands
    let listener = TcpListener::bind((server_conf.listener_addr.clone(), 33889)).unwrap();
    println!(
        "{} {}:{}",
        logger::timelog("Server is listening on"),
        server_conf.listener_addr,
        33889
    );

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let this_kg = Arc::clone(&safe_kg);
                let sd_ = Sender::clone(&sd);
                let _ = thread::spawn(move || {
                    //run handle_client and catch error if has
                    match handle_client(stream, this_kg) {
                        Err(e) => {
                            // hard check if it is suicide operation, because I only care this message so far.
                            // this operation isn't in handle_client because it has make sure return to client..
                            // ..first
                            let (first, second) = {
                                let s = e.to_string();
                                let (f, ss) = s.split_at(12);
                                (f.to_string(), ss.to_string())
                            };
                            if first == "I am dying. " {
                                println!("{}", logger::timelog(&second));
                                //tell main thread,
                                sd_.send((first.to_string(), second.to_string())).unwrap();
                            } else {
                                //if just normal error
                                println!("{}", logger::timelog(&e.to_string()));
                            }
                        }
                        Ok(des) => println!("{}", logger::timelog(&des)),
                    }
                });
            }

            Err(e) => println!("{}", logger::timelog(&e.to_string())),
        }
    }

    Ok(())
}

/// get client TCP stream and send to channel
fn handle_client(mut stream: TcpStream, kig: Arc<Mutex<Kindergarten>>) -> Result<String> {
    let mut buf = [0; 100 + 4096]; // 100 command length + 4096 key length buffer
    stream.read(&mut buf)?;

    let buf_vec = {
        let mut temp = buf.to_vec();
        temp.reverse();
        while temp[0] == 0 {
            temp.drain(..1);
        }
        temp.reverse();
        temp
    };

    // here to check if this command with
    let received_comm = if kig.lock().unwrap().encrypt_mode {
        // tell client this server running in encrypt mode
        stream.write_all("Running on encrypt mode, need to dencrypt your command.\n".as_bytes())?;

        // re-read server config
        let server_conf = if kig.lock().unwrap().server_config_path == "" {
            ServerConfig::load("/tmp/server.yml")?
        } else {
            ServerConfig::load(&kig.lock().unwrap().server_config_path)?
        };

        // parse keyname and encrypted data
        let (keyname, data) = match DataWrapper::unwrap_from(&buf_vec) {
            Ok((n, d)) => (n, d),
            Err(e) => {
                stream.write_all(e.to_string().as_bytes())?;
                return Err(e);
            }
        };

        let key = {
            let k_path = match server_conf.find_pubkey_by_name(&keyname) {
                Ok(kk) => kk,
                Err(e) => {
                    stream.write_all(e.to_string().as_bytes())?;
                    return Err(e);
                }
            };

            let mut content = String::new();
            let _ = File::open(k_path)?.read_to_string(&mut content);
            Rsa::public_key_from_pem(&content.as_bytes())?
        };

        let dw = match DataWrapper::decrypt_with_pubkey(data, keyname, key) {
            Ok(d) => d,
            Err(e) => {
                stream.write_all(e.to_string().as_bytes())?;
                return Err(e);
            }
        };

        dw.data
    } else {
        match String::from_utf8(buf_vec) {
            Ok(s) => s,
            Err(e) => return Err(ioError::new(ErrorKind::InvalidInput, e)),
        }
    };

    match day_care(kig, received_comm) {
        Ok(resp) => {
            stream.write_all(resp.as_bytes())?;
            Ok(resp)
        }
        Err(e) => {
            stream.write_all(e.to_string().as_bytes())?;
            Err(e)
        }
    }
}

/// day care is major function of server handle commands
pub fn day_care(kig: Arc<Mutex<Kindergarten>>, data: String) -> Result<String> {
    let mut kg = kig.lock().unwrap();

    // run check around here, clean all stopped children
    // check operation has its own check_around too,
    // check_around here for other operations.
    kg.check_around()?;

    let command = client::Command::new_from_str(data.as_str().split(' ').collect::<Vec<&str>>())?;

    let server_conf = if kg.server_config_path == "" {
        ServerConfig::load("/tmp/server.yml")?
    } else {
        ServerConfig::load(&kg.server_config_path)?
    };

    match command.get_ops() {
        client::Ops::Restart => {
            let name = command.child_name.as_ref().unwrap();
            // check name
            if let Err(e) = child_name_legal_check(name) {
                return Err(ioError::new(ErrorKind::InvalidInput, e));
            }

            let mut conf = server_conf.find_config_by_name(name)?;

            // check prehook here
            let pre_hook_result = server_conf.pre_hook_check(name, &conf)?;
            if !pre_hook_result.0 {
                return Err(ioError::new(
                    ErrorKind::Other,
                    format!(
                        "This child, {}, cannot pass pre-check",
                        command.child_name.unwrap()
                    ),
                ));
            } else {
                let mut pre_hook_combine = server_conf.call_chain_combine(pre_hook_result.1)?;
                // reverse call chain
                pre_hook_combine.reverse();
                kg.handle_pre_hook(pre_hook_combine)?;
            }

            match kg.restart(name, &mut conf) {
                Ok(_) => {
                    // repeat here
                    let repeat_meg = if conf.is_repeat() {
                        repeat(conf, Arc::clone(&kig), name.clone())
                    } else {
                        String::new()
                    };

                    Ok(format!(
                        "restart {} success{}",
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
                        "Cannot start this child {}, it already exsits",
                        command.child_name.unwrap()
                    ),
                ));
            }

            // read this child's config
            let mut conf = server_conf.find_config_by_name(&name)?;

            // check prehook here
            let pre_hook_result = server_conf.pre_hook_check(name, &conf)?;
            let mut pre_hook_msg = String::new();
            if !pre_hook_result.0 {
                return Err(ioError::new(
                    ErrorKind::Other,
                    format!(
                        "This child, {}, cannot pass recursive check",
                        command.child_name.unwrap()
                    ),
                ));
            } else {
                let mut pre_hook_combine = server_conf.call_chain_combine(pre_hook_result.1)?;
                // reverse call chain
                if pre_hook_combine.len() != 0 {
                    pre_hook_combine.reverse();
                    kg.handle_pre_hook(pre_hook_combine)?;
                    pre_hook_msg.push_str("Find pre-hook, has started pre-hook firstly. ");
                }
            }

            match kg.start(name, &mut conf) {
                Ok(_) => {
                    // repeat here
                    let repeat_meg = if conf.is_repeat() {
                        repeat(conf, Arc::clone(&kig), name.clone())
                    } else {
                        String::new()
                    };

                    Ok(format!(
                        "{}start {} success{}",
                        pre_hook_msg,
                        name.clone(),
                        repeat_meg
                    ))
                }
                Err(e) => Err(e),
            }
        }

        client::Ops::Stop => {
            let post_hook =
                if let Some(conf) = kg.get_child_config(command.child_name.as_ref().unwrap()) {
                    conf.get_hook(&String::from("posthook"))
                } else {
                    None
                };

            match kg.stop(command.child_name.as_ref().unwrap()) {
                Ok(_) => {
                    if let Some(post_hook_command) = post_hook {
                        // connect to supervisor itself
                        let mut stream = TcpStream::connect((
                            match server_conf.listener_addr.as_str() {
                                // listen "::" means connect to ::1
                                "::" => "::1".to_string(),

                                // listen "0.0.0.0" means connect to 127.0.0.1
                                "0.0.0.0" => "127.0.0.1".to_string(),

                                // other address
                                x => x.to_string(),
                            },
                            33889,
                        ))?;

                        stream.write_all(post_hook_command.as_bytes())?;
                        stream.flush()?;
                    }

                    return Ok(format!(
                        "stop {} success",
                        command.child_name.as_ref().unwrap()
                    ));
                }
                Err(e) => Err(e),
            }
        }

        // try start will force start child:
        // if it is running, call restart.
        // if it has stopped for some reason, start it
        client::Ops::TryStart => {
            let mut resp = String::new();

            let name = command.child_name.as_ref().unwrap();
            if let Err(e) = child_name_legal_check(name) {
                return Err(ioError::new(ErrorKind::InvalidInput, e));
            };

            //check if it is running, stop it or not.
            if let Some(_) = kg.has_child(name) {
                resp.push_str("This child already start, stop it first. ");
                let post_hook =
                    if let Some(conf) = kg.get_child_config(command.child_name.as_ref().unwrap()) {
                        conf.get_hook(&String::from("posthook"))
                    } else {
                        None
                    };
                match kg.stop(name) {
                    Ok(_) => {
                        if let Some(post_hook_command) = post_hook {
                            // connect to supervisor itself
                            let mut stream = TcpStream::connect((
                                match server_conf.listener_addr.as_str() {
                                    // listen "::" means connect to ::1
                                    "::" => "::1".to_string(),

                                    // listen "0.0.0.0" means connect to 127.0.0.1
                                    "0.0.0.0" => "127.0.0.1".to_string(),

                                    // other address
                                    x => x.to_string(),
                                },
                                33889,
                            ))?;

                            stream.write_all(post_hook_command.as_bytes())?;
                            stream.flush()?;
                            resp.push_str(&format!(
                                "find post-hook \"{}\", run it after stop. ",
                                post_hook_command
                            ));
                        }
                        resp.push_str(&format!(
                            "Stop {} success, start it again. ",
                            command.child_name.as_ref().unwrap()
                        ));
                    }
                    Err(e) => {
                        return Err(ioError::new(
                            ErrorKind::InvalidData,
                            format!("stop failed, error: {}", &e.to_string()),
                        ));
                    }
                }
            }

            let mut conf = server_conf.find_config_by_name(name)?;

            //check prehook here
            let pre_hook_result = server_conf.pre_hook_check(name, &conf)?;
            if !pre_hook_result.0 {
                return Err(ioError::new(
                    ErrorKind::Other,
                    format!(
                        "This child, {}, cannot pass pre-check",
                        command.child_name.unwrap()
                    ),
                ));
            } else {
                let mut pre_hook_combine = server_conf.call_chain_combine(pre_hook_result.1)?;
                //reverse call chain
                pre_hook_combine.reverse();
                kg.handle_pre_hook(pre_hook_combine)?;
            }

            match kg.start(name, &mut conf) {
                Ok(_) => {
                    //repeat here
                    let repeat_meg = if conf.is_repeat() {
                        repeat(conf, Arc::clone(&kig), name.clone())
                    } else {
                        String::new()
                    };

                    resp.push_str(&format!(
                        "Start {} success{} ",
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
                last_will.push_str(&format!("there is error when stop all {}", e));
            }

            // step2: return special err outside, let deamon know and stop
            Err(ioError::new(
                ErrorKind::Other,
                format!("I am dying. last error: \n{}", last_will),
            ))
        }

        client::Ops::Check => {
            kg.check_status(command.child_name.as_ref().unwrap_or(&String::new()))
        }

        client::Ops::Info => server_info(server_conf, &kg, command.child_name.as_ref()),

        _ => {
            return Err(ioError::new(
                ErrorKind::InvalidInput,
                logger::timelog("not support"),
            ))
        }
    }
}

/// receive child config, KG, and filename of child config, repeat function
fn repeat(conf: Config, kig: Arc<Mutex<Kindergarten>>, name: String) -> String {
    // clone locked val to timer
    let timer_lock_val = Arc::clone(&kig);
    let next_time = conf.to_duration().unwrap();
    let comm = conf.repeat_command().unwrap().clone();

    // give a timer
    thread::spawn(move || {
        Timer::new_from_conf(name, conf)
            .unwrap()
            .run(timer_lock_val)
    });
    format!(", and it will {} in {:?}", comm, next_time)
}

/// Server apply info of itself to client
fn server_info(config: ServerConfig, kg: &Kindergarten, name: Option<&String>) -> Result<String> {
    let name = name.map_or("all", |n| n.as_str());

    let mut resp = String::from("==Server Info Below==\n");
    match name {
        "all" => {
            // server config path
            resp.push_str("Server config path:\n");
            resp.push_str(&kg.server_config_path);
            resp.push_str("\n");

            // children name are running
            resp.push_str("Children are running (use 'check' command to see child detail):\n");
            let mut s = String::from("[");
            kg.all_running_children().iter().for_each(|name| {
                s.push_str(name);
                s.push_str(" ");
            });
            resp.push_str(&s);
            resp.push_str("]");
            resp.push_str("\n");

            // server configs
            // load paths
            resp.push_str("Server configs:\n");
            resp.push_str(&format!("Load paths: {:?}\n", config.load_paths));
            resp.push_str(&format!("Encrypt mode: {:?}\n", config.encrypt_mode));
        }
        _ => {}
    }

    resp.push_str("=======================\n");
    Ok(resp)
}

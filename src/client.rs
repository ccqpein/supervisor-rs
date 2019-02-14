use std::io::{Error, ErrorKind, Result};

#[derive(Debug)]
pub enum Ops {
    Restart,
    Stop,
    Start,
    TryStart,

    Kill,
    Check,
    None,
}

//Ops is struct of operations of client commands
impl Ops {
    fn from_str(s: &str) -> Self {
        match s {
            "Restart" | "restart" => return Ops::Restart,
            "Start" | "start" => return Ops::Start,
            "Stop" | "stop" => return Ops::Stop,
            "Check" | "check" => return Ops::Check,
            "Kill" | "kill" => return Ops::Kill,
            "TryStart" | "Trystart" | "trystart" => return Ops::TryStart,
            _ => {
                println!("does not support {}", s);
                return Ops::None;
            }
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Ops::Restart => return "restart".to_string(),
            Ops::Start => return "start".to_string(),
            Ops::Stop => return "stop".to_string(),
            Ops::Check => return "check".to_string(),
            Ops::Kill => return "kill".to_string(),
            Ops::TryStart => return "trystart".to_string(),
            _ => return "none".to_string(),
        }
    }
}

#[derive(Debug)]
pub enum Prepositions {
    On,
    None,
}

impl Prepositions {
    fn from_str(s: &str) -> Self {
        match s {
            "On" | "on" => return Prepositions::On,
            "" => {
                println!("you miss prepositions");
                return Prepositions::None;
            }
            _ => {
                println!("does not support {}", s);
                return Prepositions::None;
            }
        }
    }
}

#[derive(Debug)]
pub struct Command {
    pub op: Ops,
    pub child_name: Option<String>,
    pub prep: Option<Prepositions>,
    pub obj: Option<String>,
}

impl Command {
    pub fn new() -> Self {
        Command {
            op: Ops::None,
            child_name: None,
            prep: None,
            obj: None,
        }
    }

    pub fn new_from_string(s: Vec<String>) -> Result<Self> {
        let mut re = Self::new();

        if s.len() < 2 {
            println!("wrong");
            return Err(Error::new(ErrorKind::Other, "command parse wrong"));
        }

        re.op = Ops::from_str(&s[0]);
        re.child_name = Some(s[1].clone());

        if s.len() > 2 {
            re.prep = Some(Prepositions::from_str(&s[2]));
            if s.len() == 4 {
                re.obj = Some(s[3].clone());
            }
        }

        Ok(re)
    }

    pub fn new_from_str(s: Vec<&str>) -> Result<Self> {
        let mut re = Self::new();

        if s.len() < 2 {
            println!("Looks like command's not long enough");
            return Err(Error::new(ErrorKind::Other, "command parse wrong"));
        }

        re.op = Ops::from_str(&s[0]);
        re.child_name = Some(s[1].to_string());

        if s.len() == 4 {
            re.prep = Some(Prepositions::from_str(&s[2]));
            if s.len() == 4 {
                re.obj = Some(s[3].to_string());
            }
        }

        Ok(re)
    }
}

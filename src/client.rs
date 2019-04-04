use std::io::{Error, ErrorKind, Result};

#[derive(Debug)]
pub enum Ops {
    Restart,
    Stop,
    Start,
    TryStart,

    Kill,
    Check,
}

//Ops is struct of operations of client commands
impl Ops {
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "Restart" | "restart" => return Ok(Ops::Restart),
            "Start" | "start" => return Ok(Ops::Start),
            "Stop" | "stop" => return Ok(Ops::Stop),
            "Check" | "check" => return Ok(Ops::Check),
            "Kill" | "kill" => return Ok(Ops::Kill),
            "TryStart" | "Trystart" | "trystart" => return Ok(Ops::TryStart),
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "no legal operations input",
                ));
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
        }
    }

    pub fn is_op(s: &str) -> bool {
        if let Ok(_) = Self::from_str(s) {
            return true;
        }
        false
    }
}

#[derive(Debug)]
pub enum Prepositions {
    On,
}

impl Prepositions {
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "On" | "on" => return Ok(Prepositions::On),
            "" => {
                return Err(Error::new(ErrorKind::InvalidInput, "you miss prepositions"));
            }
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("does not support {}", s),
                ));
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
    pub fn new(op: Ops) -> Self {
        Command {
            op: op,
            child_name: None,
            prep: None,
            obj: None,
        }
    }

    pub fn new_from_string(s: Vec<String>) -> Result<Self> {
        let mut re = Self::new(Ops::from_str(&s[0])?);

        if s.len() > 1 {
            if let Ok(pre) = Prepositions::from_str(&s[1]) {
                re.prep = Some(pre);
                if s.len() > 2 {
                    re.obj = Some(s[2].clone());
                }
            } else {
                if Ops::is_op(&s[1]) {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        format!("child name cannot be command"),
                    ));
                }

                re.child_name = Some(s[1].clone());
                if s.len() > 3 {
                    re.prep = Some(Prepositions::from_str(&s[2])?);
                    re.obj = Some(s[3].clone());
                }
            }
        }

        Ok(re)
    }

    pub fn new_from_str(s: Vec<&str>) -> Result<Self> {
        let mut re = Self::new(Ops::from_str(s[0])?);

        if s.len() > 1 {
            if let Ok(pre) = Prepositions::from_str(&s[1]) {
                re.prep = Some(pre);
                if s.len() > 2 {
                    re.obj = Some(s[2].to_string());
                }
            } else {
                if Ops::is_op(&s[1]) {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        format!("child name cannot be command"),
                    ));
                }

                re.child_name = Some(s[1].to_string());
                if s.len() > 3 {
                    re.prep = Some(Prepositions::from_str(&s[2])?);
                    re.obj = Some(s[3].to_string());
                }
            }
        }

        Ok(re)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn check_child_name() {
        let case0 = vec!["restart", "restart"];
        let comm = Command::new_from_str(case0);
        dbg!(&comm);
        assert!(comm.is_err());
        assert_eq!(
            comm.err().unwrap().description(),
            "child name cannot be command"
        );
    }
}

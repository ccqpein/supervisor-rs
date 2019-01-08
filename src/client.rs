#[derive(Debug)]
pub enum Ops {
    Restart,
    Stop,
    Start,
    None, //:= MAYBE: new schdule, maybe not
}

impl Ops {
    fn from_str(s: &str) -> Self {
        match s {
            "Restart" | "restart" => return Ops::Restart,
            //:= TODO: need tell user something wrong
            _ => return Ops::None,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Ops::Restart => return "restart".to_string(),
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
            //:= TODO: need tell user something wrong
            _ => return Prepositions::None,
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

    pub fn new_from_string(s: Vec<String>) -> Self {
        let mut re = Self::new();

        if s.len() < 2 {
            println!("wrong");
            return re;
        }

        re.op = Ops::from_str(&s[0]);
        re.child_name = Some(s[1].clone());

        if s.len() > 2 {
            re.prep = Some(Prepositions::from_str(&s[2]));
            re.obj = Some(s[3].clone());
        }

        re
    }

    pub fn new_from_str(s: Vec<&str>) -> Self {
        let mut re = Self::new();

        if s.len() < 2 {
            println!("wrong");
            return re;
        }

        re.op = Ops::from_str(&s[0]);
        re.child_name = Some(s[1].to_string());

        if s.len() > 2 {
            re.prep = Some(Prepositions::from_str(&s[2]));
            re.obj = Some(s[3].to_string());
        }

        re
    }
}

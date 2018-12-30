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

    pub fn new_from_string(s: String) -> Self {
        let mut re = Self::new();
        let temp_str = s.as_str().split(' ').collect::<Vec<&str>>();

        //:= MARK: this logic should be more simple, or I just use some shell command package
        match temp_str.len() {
            //2 means ops and parameter
            2 => {
                re.op = Ops::from_str(temp_str[0]);
                re.child_name = Some(temp_str[1].to_string());
            }
            4 => {
                re.op = Ops::from_str(temp_str[0]);
                re.child_name = Some(temp_str[1].to_string());

                re.prep = Some(Prepositions::from_str(temp_str[2]));
                re.obj = Some(temp_str[3].to_string());
            }
            _ => (),
        }

        re
    }
}

//mod command;

#[derive(Debug)]
pub enum Ops {
    Restart,
    Stop,
    Start,
    None, //:= MAYBE: new schdule, maybe not
}

#[derive(Debug)]
pub struct Command {
    pub op: Ops,
    pub child_name: Option<String>,
}

impl Command {
    pub fn new_from_string(s: String) -> Self {
        let temp_str = s.as_str().split(' ').collect::<Vec<&str>>();
        match temp_str[0] {
            "Restart" | "restart" => Command {
                op: Ops::Restart,
                child_name: Some(temp_str[1].to_string()),
            },
            _ => Command {
                op: Ops::None,
                child_name: None,
            },
        }
    }
}

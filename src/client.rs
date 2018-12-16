//mod command;

enum Ops {
    Restart,
    Stop,
    Start,
    None, //:= MAYBE: new schdule, maybe not
}

struct Command {
    op: Ops,
}

impl Command {
    pub fn new_from_string(s: String) -> Self {
        match s.as_str() {
            "Restart" | "restart" => Command { op: Ops::Restart },
            _ => Command { op: Ops::None },
        }
    }
}

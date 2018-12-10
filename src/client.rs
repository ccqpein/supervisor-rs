//mod command;

enum Ops {
    Restart,
    Stop,
    Start,
    //:= MAYBE: new schdule, maybe not
}

struct Command {
    op: Ops,
}

impl Command {
    //    pub fn new_from_string(s: String) -> Self {}
}

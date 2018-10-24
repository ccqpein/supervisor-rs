use subprocess::*;

struct Config<'a> {
    Comm: &'a str,
}

fn start_new_subprocessing(comm: &str, config: &Config) {
    let exit_status = subprocess::Exec::cmd("").arg("").join()?;
}

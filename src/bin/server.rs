use std::env;
use supervisor_rs::server;

fn main() {
    let arguments = env::args();
    let change_2_vec = arguments.collect::<Vec<String>>();

    if change_2_vec.len() > 2 {
        println!("{}", "too much arguments, not support yet.");
        return;
    }

    let k = if change_2_vec.len() != 1 {
        server::start_new_server(&change_2_vec[1])
    } else {
        server::start_new_server("")
    };
    /*
    let (a, b) = match k {
        Ok(kk) => server::start_deamon(kk).unwrap(),
        Err(e) => {
            println!(
                "kindergarten build fail, looks like server cannot start, error: {}",
                e
            );
            return;
        }
    };

    a.join().unwrap();
    b.join().unwrap();*/

    server::start_deamon_2(k.unwrap()).unwrap();
}

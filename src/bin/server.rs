use supervisor_rs::server;

fn main() {
    let k = server::start_new_server();

    let (a, b) = server::start_deamon(k.unwrap()).unwrap();
    //loop {} //this will cost a lot cpu source
    a.join().unwrap();
    b.join().unwrap();
}

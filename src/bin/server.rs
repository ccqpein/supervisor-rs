use supervisor_rs::server;

fn main() {
    let k = server::start_new_server();
    println!("{:?}", k);

    let _ = server::start_deamon(k.unwrap());
    loop {}
}

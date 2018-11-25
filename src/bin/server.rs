use supervisor_rs::server;

fn main() {
    let k = server::start_new_server();
    println!("{:?}", k);

    //loop {}
}

//this package including all supervisior's need of communication
//should listen command from ouside, used by server
//should send command to server, used by client
//post is 33889

use std::io::Result;
use std::net::TcpListener;

//open a listener and return
fn open_listener(host: &str, port: &str) -> Result<TcpListener> {
    TcpListener::bind(format!("{}:{}", host, port))
}

//client call servers
fn call_server() {}

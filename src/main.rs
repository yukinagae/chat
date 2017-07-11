extern crate mio;

use mio::*;
use mio::tcp::{TcpListener};

use std::net::SocketAddr;

const SERVER: Token = Token(0);

fn main() {
    let poll = Poll::new().unwrap();
    // let mut handler = WebSocketServer;
    // event_loop.run(&mut handler);

    let address = "0.0.0.0:10000".parse::<SocketAddr>().unwrap();
    let server_socket = TcpListener::bind(&address).unwrap();

    poll.register(
        &server_socket,
        SERVER,
        Ready::readable(),
        PollOpt::edge()
    ).unwrap();

    let mut events = Events::with_capacity(1024);

    loop {
        poll.poll(&mut events, None).unwrap();

        for event in events.iter() {
            match event.token() {
                SERVER => {
                    let _ = server_socket.accept();
                }
                _ => unreachable!(),
            }
        }
    }
}

// struct WebSocketServer;

// impl Handler for WebSocketServer {
//     type Timeout = usize;
//     type Message = ();
// }

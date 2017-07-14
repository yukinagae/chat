use std::io;
use mio::*;
use mio::tcp::{TcpListener, TcpStream};
use std::collections::HashMap;

use client::WebSocketClient;

//
// config
//
pub const SERVER: Token = Token(0);

//
// server
//
pub struct WebSocketServer {
    pub socket: TcpListener,
    pub token_counter: usize,
    pub clients: HashMap<Token, WebSocketClient>,
}

impl WebSocketServer {
    pub fn new(server_socket: TcpListener) -> Self {
        WebSocketServer { socket: server_socket, token_counter: 1, clients: HashMap::new() }
    }
}

impl Evented for WebSocketServer {

    fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
        println!("register event");
        self.socket.register(poll, token, interest, opts)
    }

    fn reregister(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
        println!("re-register event");
        self.socket.reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        println!("de-register event");
        self.socket.deregister(poll)
    }
}

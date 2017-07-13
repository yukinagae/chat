extern crate mio;
extern crate http_muncher;

use mio::*;
use mio::tcp::{TcpListener, TcpStream};
use std::net::SocketAddr;
use std::io;
use std::collections::HashMap;

use http_muncher::{Parser, ParserHandler};

use std::io::Read;

// const
const SERVER: Token = Token(0);


// main
fn main() {

    let address = "0.0.0.0:10000".parse::<SocketAddr>().unwrap();
    let server_socket = TcpListener::bind(&address).unwrap();

    let mut server = WebSocketServer::new(server_socket);

    let poll = Poll::new().unwrap();
    poll.register(
        &server,
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
                    let client_socket = match server.socket.accept() {
                        Err(e) => { println!("Accept error: {}", e); return; },
                        Ok((sock, _)) => sock
                    };
                    server.token_counter += 1;
                    let new_token = Token(server.token_counter);
                    server.clients.insert(new_token, WebSocketClient::new(client_socket));

                    println!("{:?}", new_token);

                    poll.register(
                        &server.clients[&new_token].socket,
                        new_token,
                        Ready::readable(),
                        PollOpt::edge() | PollOpt::oneshot()
                    ).unwrap();
                }
                token => {
                    let mut client = server.clients.get_mut(&token).unwrap();
                    let a = client.read();
                    println!("{:?}", a);
                    poll.reregister(
                        &client.socket,
                        token,
                        Ready::readable(),
                        PollOpt::edge() | PollOpt::oneshot()
                    ).unwrap();
                },
            }
        }
    }
}

struct WebSocketServer {
    socket: TcpListener,
    token_counter: usize,
    clients: HashMap<Token, WebSocketClient>,
}

impl WebSocketServer {
    fn new(server_socket: TcpListener) -> Self {
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

struct HttpParser;
impl ParserHandler for HttpParser {}

struct WebSocketClient {
    socket: TcpStream,
    http_parser: Parser,
}

impl WebSocketClient {

    fn new(socket: TcpStream) -> Self {
        WebSocketClient { socket: socket, http_parser: Parser::request() }
    }

    fn read(&mut self) {
        loop {
            let mut buf = [0; 2048];
            match self.socket.read(&mut buf) {
                Err(e) => {println!("Error while reading socket: {:?}", e); return; },
                Ok(len) => {
                    self.http_parser.parse(&mut HttpParser {}, &buf[0..len]);
                    if self.http_parser.is_upgrade() {
                        // TODO:
                        break;
                    }
                }
            }
        }
    }
}
extern crate mio;
extern crate http_muncher;
extern crate sha1;
extern crate rustc_serialize;

use mio::*;
use mio::tcp::{TcpListener, TcpStream};
use std::net::SocketAddr;
use std::io;
use std::collections::HashMap;
use http_muncher::{Parser, ParserHandler};
use std::io::Read;
use std::io::Write;
use std::str;
use rustc_serialize::base64::{ToBase64, STANDARD};
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt;

//
// config
//
const SERVER: Token = Token(0);

//
// main
//
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

    // event loop
    loop {
        poll.poll(&mut events, None).unwrap();

        for event in events.iter() {
            let token = event.token();
            let ready = event.readiness();

            // when read
            if ready.is_readable() {
                println!("readable");
                match token {
                    // first connection
                    SERVER => {
                        let client_socket = match server.socket.accept() {
                            Err(e) => { println!("Accept error: {}", e); return; },
                            Ok((sock, _)) => sock
                        };
                        println!("{:?}", client_socket);
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
                    // client per connection
                    token => {
                        let mut client = server.clients.get_mut(&token).unwrap();
                        client.read();
                        poll.reregister(
                            &client.socket,
                            token,
                            client.interest,
                            PollOpt::edge() | PollOpt::oneshot()
                        ).unwrap();
                    },
                }
            }

            // when write
            if ready.is_writable() {
                println!("writable");
                let mut client = server.clients.get_mut(&token).unwrap();
                client.write();
                poll.reregister(
                    &client.socket,
                    token,
                    client.interest,
                    PollOpt::edge() | PollOpt::oneshot()
                ).unwrap();
            }
        }
    }
}

//
// server
//
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

//
// client
//
#[derive(Debug)]
struct HttpParserHandler {
    current_key: Option<String>,
    headers: Rc<RefCell<HashMap<String, String>>>,
}

impl ParserHandler for HttpParserHandler {

    fn on_header_field(&mut self, parser: &mut Parser, s: &[u8]) -> bool {
        self.current_key = Some(str::from_utf8(s).unwrap().to_string());
        true
    }

    fn on_header_value(&mut self, parser: &mut Parser, s: &[u8]) -> bool {
        self.headers.borrow_mut().insert(
            self.current_key.clone().unwrap(),
            str::from_utf8(s).unwrap().to_string()
        );
        true
    }

    fn on_headers_complete(&mut self, parser: &mut Parser) -> bool {
        false
    }
}

// generate key
fn gen_key(key: &String) -> String {
    let mut m = sha1::Sha1::new();
    m.update(key.as_bytes());
    m.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    return m.digest().bytes().to_base64(STANDARD)
}

#[derive(Debug)]
struct WebSocketClient {
    socket: TcpStream,
    headers: Rc<RefCell<HashMap<String, String>>>,
    handler: HttpParserHandler,
    interest: Ready,
    state: ClientState,
}

impl WebSocketClient {

    // initial state
    fn new(socket: TcpStream) -> Self {
        let headers = Rc::new(RefCell::new(HashMap::new()));
        let handler = HttpParserHandler { current_key: None, headers: headers.clone() };
        WebSocketClient {
            socket: socket,
            headers: headers.clone(),
            handler: handler,
            interest: Ready::readable(),
            state: ClientState::AwaitingHandshake,
        }
    }

    fn read(&mut self) {
        loop {
            let mut buf = [0; 2048];
            match self.socket.read(&mut buf) {
                Err(e) => { println!("Error while reading socket: {:?}", e); return; },
                Ok(len) => {
                    println!("{:?}", str::from_utf8(&buf[0..len]));
                    let mut parser = Parser::request();
                    parser.parse(&mut self.handler, &buf[0..len]);
                    if parser.has_error() {
                        println!("Error while reading http: {:?}", parser.error());
                        return;
                    }
                    // websocket protocol
                    if parser.is_upgrade() {
                        self.state = ClientState::HandshakeResponse;
                        self.interest.remove(Ready::readable());
                        self.interest.insert(Ready::writable()); // now writable
                        break;
                    }
                }
            }
        }
    }

    fn write(&mut self) {
        let headers = self.headers.borrow();
        let response_key = gen_key(&headers.get("Sec-WebSocket-Key").unwrap());
        let response = fmt::format(format_args!("HTTP/1.1 101 Switching Protocols\r\n\
                                                 Connection: Upgrade\r\n\
                                                 Sec-WebSocket-Accept: {}\r\n\
                                                 Upgrade: websocket\r\n\r\n", response_key));
        self.socket.write(response.as_bytes()).unwrap();
        self.state = ClientState::Connected;
        self.interest.remove(Ready::writable());
        self.interest.insert(Ready::readable());
    }
}

#[derive(PartialEq, Debug)]
enum ClientState {
    AwaitingHandshake,
    HandshakeResponse,
    Connected,
}

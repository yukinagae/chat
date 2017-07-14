use sha1;
use std::collections::HashMap;
use mio::*;
use std::cell::RefCell;
use std::rc::Rc;
use mio::tcp::TcpStream;
use rustc_serialize::base64::{ToBase64, STANDARD};
use std::fmt;
use http_muncher::{Parser, ParserHandler};
use handler::HttpParserHandler;
use std::str;
use std::io::Read;
use std::io::Write;
use frame::WebSocketFrame;
use frame::OpCode;

#[derive(Debug)]
pub struct WebSocketClient {
    pub socket: TcpStream,
    pub headers: Rc<RefCell<HashMap<String, String>>>,
    pub handler: HttpParserHandler,
    pub interest: Ready,
    pub state: ClientState,
}

impl WebSocketClient {

    // initial state
    pub fn new(socket: TcpStream) -> Self {
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

    pub fn read(&mut self) {
        match self.state {
            ClientState::AwaitingHandshake => self.read_handshake(),
            ClientState::Connected => self.read_frame(),
            _ => {},
        }
    }

    pub fn read_frame(&mut self) {
        let frame = WebSocketFrame::read(&mut self.socket);
        match frame {
            Ok(frame) => {
                match frame.get_opcode() {
                    OpCode::TextFrame => {
                        println!("{:?}", frame);
                        let payload = String::from_utf8(frame.payload).unwrap();
                        println!("{:?}", payload);
                    },
                    OpCode::BinaryFrame => {

                    },
                    OpCode::Ping => {

                    },
                    OpCode::ConnectionClose => {

                    },
                    _ => {}
                }

                // TODO: self outgoing
            }
            Err(e) => println!("error while reading frame: {}", e)
        }
    }

    pub fn read_handshake(&mut self) {
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

                    let is_upgrade = if let ClientState::AwaitingHandshake = self.state {
                        parser.is_upgrade()
                    } else {
                        false
                    };

                    // websocket protocol
                    if is_upgrade {
                        self.state = ClientState::HandshakeResponse;
                        self.interest.remove(Ready::readable());
                        self.interest.insert(Ready::writable()); // now writable
                        break;
                    }
                }
            }
        }
    }

    pub fn write(&mut self) {
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
pub enum ClientState {
    AwaitingHandshake,
    HandshakeResponse,
    Connected,
}

// generate key
fn gen_key(key: &String) -> String {
    let mut m = sha1::Sha1::new();
    m.update(key.as_bytes());
    m.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    return m.digest().bytes().to_base64(STANDARD)
}

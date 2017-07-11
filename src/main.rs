extern crate mio;

use mio::*;
use mio::tcp::TcpStream;

use std::net::SocketAddr;
use std::io;

const SERVER: Token = Token(0);

fn main() {
    let poll = Poll::new().unwrap();
    // let mut handler = WebSocketServer;
    // event_loop.run(&mut handler);

    let address = "0.0.0.0:10000".parse::<SocketAddr>().unwrap();
    let server_socket = TcpStream::connect(&address).unwrap();

    let server = WebSocketServer { socket: server_socket };

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
                    // TODO: do something
                }
                _ => unreachable!(),
            }
        }
    }
}

struct WebSocketServer {
    socket: TcpStream,
}

impl Evented for WebSocketServer {

    fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
        // Delegate the `register` call to `socket`
        self.socket.register(poll, token, interest, opts)
    }

    fn reregister(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
        // Delegate the `reregister` call to `socket`
        self.socket.reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        // Delegate the `deregister` call to `socket`
        self.socket.deregister(poll)
    }
}

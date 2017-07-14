extern crate mio;
extern crate http_muncher;
extern crate sha1;
extern crate rustc_serialize;
extern crate byteorder;

mod server;
mod client;
mod handler;
mod frame;

use mio::*;
use mio::tcp::TcpListener;
use std::net::SocketAddr;

use server::WebSocketServer;
use client::WebSocketClient;

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
        server::SERVER,
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
                    server::SERVER => {
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

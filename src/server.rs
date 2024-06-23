use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};
use std::collections::HashMap;
use std::io::ErrorKind::{Interrupted, WouldBlock};
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::str::from_utf8;

struct Client {
    connection: TcpStream,
    address: SocketAddr,
}

struct Server {
    token: Token,
    clients: HashMap<Token, Client>,
}

impl Server {
    fn new() -> Server {
        Server {
            token: Token(0),
            clients: HashMap::new(),
        }
    }

    fn client_connected(&mut self, token: Token, connection: TcpStream, address: SocketAddr) {
        println!("Client connected: {address}");

        self.clients.insert(
            token,
            Client {
                connection,
                address,
            },
        );
    }

    fn handle_user_message(&mut self, token: Token) {
        if let Some(client) = self.clients.get_mut(&token) {
            let mut buf = [0; 64];
            let bytes: Vec<_> = match client.connection.read(&mut buf) {
                Ok(0) => {
                    println!("Client disconnected: {}", client.address);
                    self.clients.remove(&token);
                    return;
                }
                Ok(n) => buf[0..n].iter().cloned().filter(|x| *x >= 32).collect(),
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::WouldBlock {
                        eprintln!("Could not read message from client {}: {e}", client.address);
                        self.clients.remove(&token);
                    }
                    return;
                }
            };

            let msg = if let Ok(msg) = from_utf8(&bytes) {
                msg
            } else {
                return;
            };

            println!("New message from {}: '{}'", client.address, msg);
            self.broadcast_message(token, msg.as_bytes());
        }
    }

    fn broadcast_message(&mut self, token: Token, message: &[u8]) {
        for (tok, client) in self.clients.iter_mut() {
            if token != *tok {
                let _ = client.connection.write_all(message).map_err(|e| {
                    eprintln!("Failed to write message into the client: {e}");
                });
            }
        }
    }
}

fn main() -> io::Result<()> {
    let mut poll = Poll::new()?;

    let mut events = Events::with_capacity(1024);

    let addr = "0.0.0.0:1337".parse().unwrap();
    let mut listener = TcpListener::bind(addr)?;

    let mut server = Server::new();

    poll.registry()
        .register(&mut listener, server.token, Interest::READABLE)?;

    let mut num = server.token.0;

    loop {
        if let Err(e) = poll.poll(&mut events, None) {
            if e.kind() == Interrupted {
                continue;
            }
            return Err(e);
        }
        for token in events.iter().map(|ev| ev.token()) {
            match token {
                Token(0) => match listener.accept() {
                    Ok((mut connection, addr)) => {
                        num += 1;
                        let token = Token(num);
                        match poll
                            .registry()
                            .register(&mut connection, token, Interest::READABLE)
                        {
                            Ok(_) => server.client_connected(token, connection, addr),
                            Err(e) => eprintln!("Error: {e}"),
                        }
                    }
                    Err(e) if e.kind() == WouldBlock => break,
                    Err(e) => return Err(e),
                },
                token => server.handle_user_message(token),
            }
        }
    }
}

#![allow(unused_variables)]

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc::channel;
use std::thread;

const ADDR: &str = "0.0.0.0:1337";

fn main() {
    let listener = TcpListener::bind(ADDR).expect("Failed to bind to address {ADDR}");
    listener
        .set_nonblocking(true)
        .expect("set_nonblocking failed");

    let (sender, receiver) = channel::<(String, SocketAddr)>();
    let mut clients: Vec<(TcpStream, SocketAddr)> = Vec::new();
    loop {
        if let Ok((mut socket, addr)) = listener.accept() {
            println!("New client connected: {addr}");
            clients.push((socket.try_clone().expect("Failed to clone socket"), addr));

            let sender = sender.clone();

            thread::spawn(move || loop {
                let mut buf: [u8; 64] = [0; 64];
                let bytes: Vec<_> = match socket.read(&mut buf) {
                    Ok(0) => {
                        println!("Client disconnected: {addr}");
                        break;
                    }
                    Ok(n) => buf[0..n].iter().cloned().filter(|x| *x >= 32).collect(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::WouldBlock {
                            eprintln!("Could not read message from client {addr}: {err}");
                            continue;
                        }
                        return;
                    }
                };
                let msg: String = if let Ok(msg) = std::str::from_utf8(&bytes) {
                    msg.to_string()
                } else {
                    return;
                };
                println!("New message received from client {addr}: '{msg}'");

                sender.send((msg, addr)).expect("Failed to send message");
            });
        }

        if let Ok((msg, addr)) = receiver.try_recv() {
            let msg = msg.as_bytes();
            for (ref mut client, caddr) in clients.iter_mut() {
                if *caddr != addr {
                    client
                        .write_all(&msg)
                        .expect("Failed to write message into the client");
                }
            }
        }
    }
}

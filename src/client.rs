use std::env::args;
use std::io::ErrorKind::WouldBlock;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::process::exit;
use std::thread;

fn main() {
    let args: Vec<String> = args().collect();
    if args.len() < 3 {
        println!("Please provide an address and a port to connect to");
        exit(-1);
    }

    let addr: String = format!("{}:{}", args[1], args[2]);

    let mut server = TcpStream::connect(addr.clone()).expect("Failed to connect to the server");
    server
        .set_nonblocking(true)
        .expect("set_nonblocking failed");
    println!("Connected to {}", addr);

    let mut username = String::new();
    println!("Enter your username: ");
    io::stdin()
        .read_line(&mut username)
        .expect("Failed to read input");

    let mut cloned_server = server.try_clone().expect("Failed to clone server");

    thread::spawn(move || loop {
        let mut buf: [u8; 64] = [0; 64];
        let bytes: Vec<_> = match cloned_server.read(&mut buf) {
            Ok(0) => {
                println!("Disconnected from the server");
                exit(-1);
            }
            Ok(n) => buf[0..n].iter().cloned().filter(|x| *x >= 32).collect(),
            Err(err) => {
                if err.kind() == WouldBlock {
                    continue;
                }
                return;
            }
        };

        if let Ok(msg) = std::str::from_utf8(&bytes) {
            println!("{}", msg);
        }
    });

    loop {
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read input");
        server
            .write_all(format!("{username}: {input}").as_bytes())
            .expect("Failed to send message to server");
    }
}

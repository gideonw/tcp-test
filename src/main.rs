use anyhow::{anyhow, Result};
// use std::collections::HashMap;
// use futures::lock::Mutex;
use std::convert::{From, TryFrom};
use std::env;
use std::io::Cursor;
use std::sync::{Arc, RwLock};
use std::thread;
use tokio::io::{Interest, Ready};
use tokio::join;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Mutex;
use tokio::task::spawn;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.iter().count() > 1 {
        client().await;
    } else {
        server().await;
    }
}

async fn client() {
    let con = TcpStream::connect("localhost:8080").await.ok().unwrap();
    loop {
        con.writable().await;
        match con.try_write(b"A") {
            Ok(n) => {
                println!("write {} bytes", n);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
            Err(e) => {
                println!("{}", e)
            }
        }
        thread::sleep(std::time::Duration::from_millis(100));
    }
}

async fn server() {
    let listener = match TcpListener::bind("localhost:8080").await {
        Ok(l) => Some(l),
        Err(e) => {
            panic!("{}", e);
        }
    };
    let l_listner = listener.unwrap();
    loop {
        match l_listner.accept().await {
            Ok((socket, addr)) => {
                println!("Addr {}", addr);
                spawn(async move {
                    let addr = addr;
                    loop {
                        let mut data = vec![0; 1024];
                        match socket.try_read(&mut data) {
                            Ok(0) => {
                                println!("disconn {} ", addr);
                                break;
                            }
                            Ok(n) => {
                                println!("read {} bytes", n);
                            }
                            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                continue;
                            }
                            Err(e) => {
                                println!("{}", e)
                            }
                        }
                    }
                });
            }
            Err(_) => println!("huih"),
        }
    }
}

enum CommandFrame {
    Hello { cmd: u8, payload: Vec<u8> },
}

struct Connection {
    stream: TcpStream,
    buffer: Vec<u8>,
    cursor: usize,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            buffer: vec![0; 1024],
            cursor: 0,
        }
    }

    pub fn read_frame(&self) -> Option<CommandFrame> {
        let mut cursor = Cursor::new(&self.buffer);

        for i in cursor.read(buf: &mut [u8]) {}

        None
    }
}

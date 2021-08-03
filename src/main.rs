use anyhow::Result;
// use std::collections::HashMap;
use std::env;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex, RwLock};
use std::thread;
use tokio::io::{AsyncReadExt, AsyncWriteExt, Interest};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::{spawn, JoinHandle};

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.iter().count() > 1 {
        let b = Broker::new();
        let c_b = b.clone();
        let th = spawn(async move { c_b.process().await });

        if let Err(e) = b.listen("localhost:8080").await {
            panic!("Error listening {}", e)
        }

        let _ = th.await;
    } else {
        let mut e = Executor::connect("localhost:8080".to_string())
            .await
            .unwrap();

        let th = tokio::spawn(async move { e.handle_tcp().await });
        let _ = th.await;
    }
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

struct ExecutorThread {
    // todo
}

impl ExecutorThread {
    // todo
}

struct Executor {
    cmds_in: CommandBuffer,
    cmds_out: CommandBuffer,
    socket: TcpStream,
}

impl Executor {
    async fn connect(addr: String) -> Result<Self> {
        if let Ok(socket) = TcpStream::connect(addr).await {
            Ok(Self {
                cmds_in: CommandBuffer::new(),
                cmds_out: CommandBuffer::new(),
                socket: socket,
            })
        } else {
            panic!("Error connecting")
        }
    }

    async fn handle_tcp(&mut self) {
        loop {
            // Recieve
            match self
                .socket
                .ready(Interest::READABLE | Interest::WRITABLE)
                .await
            {
                Ok(ready) if ready.is_readable() => {
                    let mut data = vec![0; 1024];
                    match self.socket.try_read(&mut data) {
                        Ok(0) => break,
                        Ok(n) => {
                            println!("read {} bytes", n);
                        }
                        Err(_) => continue,
                    }
                }
                Ok(ready) if ready.is_writable() => match self.socket.try_write(b"hello world") {
                    Ok(n) => {
                        println!("write {} bytes", n);
                    }
                    Err(_) => continue,
                },
                Ok(_) => {
                    println!("Unknown ok pattern")
                }
                Err(_) => {
                    println!("Unknown error pattern")
                }
            }
            thread::sleep(std::time::Duration::from_millis(1000));
        }
    }

    fn process(&mut self) {
        loop {}
    }
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
#[derive(Debug)]
struct BrokerDispatcher {
    cmd_stack: CommandStack,
    socket: TcpStream,
}

impl BrokerDispatcher {
    pub async fn handle_tcp(self) {
        loop {
            // Recieve
            match self
                .socket
                .ready(Interest::READABLE | Interest::WRITABLE)
                .await
            {
                Ok(ready) if ready.is_readable() => {
                    let mut data = vec![0; 1024];
                    match self.socket.try_read(&mut data) {
                        Ok(0) => break,
                        Ok(n) => {
                            println!("read {} bytes", n);
                        }
                        Err(_) => continue,
                    }
                }
                Ok(ready) if ready.is_writable() => match self.socket.try_write(b"hello world") {
                    Ok(n) => {
                        println!("write {} bytes", n);
                    }
                    Err(_) => continue,
                },
                Ok(_) => {
                    println!("Unknown ok pattern")
                }
                Err(e) => {
                    println!("Unknown error pattern {}", e)
                }
            }
            thread::sleep(std::time::Duration::from_millis(1000));
        }
    }
}

// type ExecutorList = Arc<RwLock<Vec<BrokerDispatcher>>>;

#[derive(Debug, Clone)]
struct Broker {
    // connections: ExecutorList,
    tasks: Arc<Vec<JoinHandle<()>>>,
}

impl Broker {
    fn new() -> Self {
        Self {
            // connections: Arc::new(RwLock::new(Vec::new())),
            tasks: Arc::new(Vec::new()),
        }
    }
    async fn listen(&self, addr: &str) -> Result<()> {
        let listener = TcpListener::bind(addr).await?;
        println!("Listening on {}", addr);

        loop {
            match listener.accept().await {
                Ok((socket, client_addr)) => {
                    println!("New connection on {}", client_addr);
                    let (in_send, in_recv) = channel();
                    let (out_send, out_recv) = channel();
                    let c_in_recv = Arc::new(Mutex::new(in_recv));

                    let broker_disp = BrokerDispatcher {
                        cmd_stack: CommandStack::new(c_in_recv.clone(), out_send),
                        socket: socket,
                    };
                    // let c_broker_disp = broker_disp.clone();
                    // self.connections.write().unwrap().push(broker_disp);
                    spawn(async move { broker_disp.handle_tcp().await });
                    // self.tasks.push(new_task);
                }
                Err(e) => {
                    println!("Error accepting connection: {}", e)
                }
            }
        }
    }
    async fn process(&self) {
        loop {
            println!("Proc");
            // loops over conns and process commands in and out
            for conn in self.tasks.iter() {
                // conn
                // if let Some(cmd) = conn. {
                //     println!("CMD IN: {:?}", cmd);
                // }

                // conn.cmd_stack.push_out(Command {
                //     cmd: b'A',
                //     bytes: Vec::new(),
                // });
            }
            thread::sleep(std::time::Duration::from_millis(1000));
        }
    }
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

#[derive(Debug)]
struct Command {
    cmd: u8,
    bytes: Vec<u8>,
}

#[derive(Debug)]
struct CommandBuffer {
    queue: Vec<Command>,
}

impl CommandBuffer {
    pub fn new() -> Self {
        Self {
            queue: Vec::with_capacity(10),
        }
    }

    pub fn push(&mut self, cmd: Command) {
        self.queue.push(cmd)
    }

    pub fn pop(&mut self) -> Option<Command> {
        self.queue.pop()
    }
}

/// Thread safe command stack
#[derive(Debug)]
struct CommandStack {
    cmds_in: Arc<Mutex<Receiver<Command>>>,
    cmds_out: Sender<Command>,
}

impl CommandStack {
    pub fn new(cmds_in: Arc<Mutex<Receiver<Command>>>, cmds_out: Sender<Command>) -> Self {
        Self { cmds_in, cmds_out }
    }

    pub fn recv(self, cmd: Command) -> Option<Command> {
        match self.cmds_in.lock().unwrap().recv() {
            Ok(cmd) => Some(cmd),
            Err(e) => {
                println!("Unable to recv from Command Stack {}", e);
                None
            }
        }
    }
    pub fn send(self, cmd: Command) {
        match self.cmds_out.send(cmd) {
            Ok(_) => {}
            Err(e) => println!("Unable to send to Command Stack {}", e),
        }
    }
}

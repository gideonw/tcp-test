use anyhow::Result;
// use std::collections::HashMap;
use std::env;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use tokio::io::{AsyncReadExt, AsyncWriteExt, Interest};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.iter().count() > 1 {
        let b = Broker::new();
        let c_b = b.clone();
        let th = tokio::spawn(async move {
            c_b.process();
        });

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
            thread::sleep_ms(1000);
        }
    }

    fn process(&mut self) {
        loop {}
    }
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
#[derive(Clone, Debug)]
struct BrokerDispatcher {
    cmd_stack: CommandStack,
    socket: Arc<TcpStream>,
}

impl BrokerDispatcher {
    pub async fn handle_tcp(&self) {
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
            thread::sleep_ms(1000);
        }
    }
}

type ExecutorList = Arc<RwLock<Vec<Arc<BrokerDispatcher>>>>;

#[derive(Clone)]
struct Broker {
    connections: ExecutorList,
}

impl Broker {
    fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(Vec::new())),
        }
    }
    async fn listen(&self, addr: &str) -> Result<()> {
        let listener = TcpListener::bind(addr).await?;
        println!("Listening on {}", addr);

        loop {
            match listener.accept().await {
                Ok((socket, client_addr)) => {
                    println!("New connection on {}", client_addr);
                    let broker_disp = Arc::new(BrokerDispatcher {
                        cmd_stack: CommandStack::new(),
                        socket: Arc::new(socket),
                    });
                    let c_broker_disp = broker_disp.clone();
                    self.connections.write().unwrap().push(broker_disp);
                    tokio::spawn(async move { c_broker_disp.handle_tcp().await });
                }
                Err(e) => {
                    println!("Error accepting connection: {}", e)
                }
            }
        }
    }
    fn process(&self) {
        loop {
            println!("Proc");
            // loops over conns and process commands in and out
            for conn in self.connections.write().unwrap().iter_mut() {
                if let Some(cmd) = conn.cmd_stack.pop_in() {
                    println!("CMD IN: {:?}", cmd);
                }

                conn.cmd_stack.push_out(Command {
                    cmd: b'A',
                    bytes: Vec::new(),
                });
            }
            thread::sleep_ms(1000);
        }
    }
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct Command {
    cmd: u8,
    bytes: Vec<u8>,
}

#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
struct CommandStack {
    cmds_in: Arc<RwLock<CommandBuffer>>,
    cmds_out: Arc<RwLock<CommandBuffer>>,
}

impl CommandStack {
    pub fn new() -> Self {
        Self {
            cmds_in: Arc::new(RwLock::new(CommandBuffer::new())),
            cmds_out: Arc::new(RwLock::new(CommandBuffer::new())),
        }
    }

    pub fn push_in(&mut self, cmd: Command) {
        self.cmds_in.write().unwrap().push(cmd)
    }
    pub fn push_out(&mut self, cmd: Command) {
        self.cmds_out.write().unwrap().push(cmd)
    }

    pub fn pop_in(&mut self) -> Option<Command> {
        self.cmds_in.write().unwrap().pop()
    }
    pub fn pop_out(&mut self) -> Option<Command> {
        self.cmds_out.write().unwrap().pop()
    }
}

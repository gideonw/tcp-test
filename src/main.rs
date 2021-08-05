use anyhow::{anyhow, Result};
// use std::collections::HashMap;
// use futures::lock::Mutex;
use std::convert::{From, TryFrom};
use std::env;
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
        let b = Arc::new(Broker::new());
        let c_b = b.clone();
        let listen = b.listen("localhost:8080");
        let h = spawn(async move { c_b.clone().process().await });

        if let Err(e) = listen.await {
            panic!("Error listening {}", e)
        }
        join!(h);
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
                Ok(ready) if ready.is_writable() => match self.socket.try_write(b"Atest") {
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
            // thread::sleep(std::time::Duration::from_millis(50));
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
    // async fn get_next_cmd(&mut self) -> Option<Command> {
    //     self.cmd_stack.recv()
    // }

    pub async fn socket_ready(&self) -> Result<Ready, std::io::Error> {
        self.socket
            .ready(Interest::READABLE | Interest::WRITABLE)
            .await
    }

    pub async fn socket_try_read(&self, buf: &mut Vec<u8>) -> Result<usize, std::io::Error> {
        self.socket.try_read(buf)
    }

    pub async fn socket_try_write(&self, buf: &Vec<u8>) -> Result<usize, std::io::Error> {
        self.socket.try_write(buf)
    }

    pub async fn send_command(&self, cmd: Command) {
        self.cmd_stack.send(cmd).await
    }

    pub async fn recv_command(&mut self) -> Option<Command> {
        self.cmd_stack.recv().await
    }
    pub async fn handle_tcp(mut self) {
        loop {
            println!("tcp - loop");
            // Recieve
            match self.socket_ready().await {
                Ok(ready) if ready.is_readable() => {
                    println!("tcp - read");

                    let mut data: Vec<u8> = vec![0; 1024];
                    match self.socket_try_read(&mut data).await {
                        Ok(0) => break,
                        Ok(n) => {
                            println!("read {} bytes", n);
                            match Command::try_from(data) {
                                Ok(cmd) => {
                                    println!("tcp - send");
                                    self.send_command(cmd).await
                                }
                                Err(_) => continue,
                            }
                        }
                        Err(_) => continue,
                    }
                }
                Ok(ready) if ready.is_writable() => {
                    println!("tcp - recv");
                    if let Some(cmd) = self.recv_command().await {
                        let w_cmd: Vec<u8> = Vec::from(cmd);
                        println!("tcp - write");
                        match self.socket_try_write(&w_cmd).await {
                            Ok(n) => {
                                println!("write {} bytes", n);
                            }
                            Err(_) => continue,
                        }
                    }
                }
                Ok(_) => {
                    println!("Unknown ok pattern")
                }
                Err(e) => {
                    println!("Unknown error pattern {}", e)
                }
            }
            println!("tcp - sleep");
            // thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}

type ExecutorList = Arc<Mutex<Vec<CommandStack>>>;

#[derive(Debug, Clone)]
struct Broker {
    connections: ExecutorList,
    // tasks: Arc<Vec<JoinHandle<()>>>,
}

impl Broker {
    fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(Vec::new())),
            // tasks: Arc::new(Vec::new()),
        }
    }
    async fn listen(&self, addr: &str) -> Result<()> {
        let listener = TcpListener::bind(addr).await?;
        println!("Listening on {}", addr);

        loop {
            match listener.accept().await {
                Ok((socket, client_addr)) => {
                    println!("New connection on {}", client_addr);
                    let (in_send, in_recv) = channel(64);
                    let (out_send, out_recv) = channel(64);
                    // let c_in_recv = Arc::new(Mutex::new(in_recv));
                    // let c_out_recv = Arc::new(Mutex::new(out_recv));

                    let broker_disp = BrokerDispatcher {
                        cmd_stack: CommandStack::new(in_recv, out_send),
                        socket: socket,
                    };
                    self.connections
                        .lock()
                        .await
                        .push(CommandStack::new(out_recv, in_send));
                    spawn(async move { broker_disp.handle_tcp().await });
                    // spawn(async move {
                    //     loop {
                    //         match broker_disp.socket_ready().await {
                    //             Ok(_) => {
                    //                 let mut data = vec![0; 1024];
                    //                 match broker_disp.socket_try_read(&mut data).await {
                    //                     Ok(_) => {
                    //                         broker_disp
                    //                             .send_command(Command {
                    //                                 cmd: b'A',
                    //                                 bytes: data,
                    //                             })
                    //                             .await
                    //                     }
                    //                     Err(_) => continue,
                    //                 }
                    //             }
                    //             Err(_) => {}
                    //         }
                    //     }
                    // });
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
            {
                let mut conns = self.connections.lock().await;
                println!("Proc - lock conn");

                for conn in conns.iter_mut() {
                    if let Some(cmd) = conn.recv().await {
                        println!("CMD IN: {:?}", cmd);
                    }
                    conn.send(Command {
                        cmd: b'A',
                        bytes: Vec::new(),
                    })
                    .await;
                }
            }
            println!("Proc - sleep");

            // thread::sleep(std::time::Duration::from_millis(10));
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

impl TryFrom<Vec<u8>> for Command {
    type Error = anyhow::Error;

    fn try_from(v: Vec<u8>) -> Result<Command> {
        if v.len() <= 1 {
            return Err(anyhow!("Missing attribute:"));
        }
        let ret = Command {
            cmd: v[0],
            bytes: v[1..].to_vec(),
        };
        Ok(ret)
    }
}

impl From<Command> for Vec<u8> {
    fn from(v: Command) -> Vec<u8> {
        let mut ret = vec![v.cmd];
        for i in v.bytes.iter() {
            ret.push(*i)
        }
        ret
    }
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
    cmds_in: Receiver<Command>,
    cmds_out: Sender<Command>,
}

impl CommandStack {
    pub fn new(cmds_in: Receiver<Command>, cmds_out: Sender<Command>) -> Self {
        Self { cmds_in, cmds_out }
    }

    pub async fn recv(&mut self) -> Option<Command> {
        self.cmds_in.recv().await
        // self.cmds_in.recv().await
        //  {
        //     Ok(cmd) => Some(cmd),
        //     Err(e) => {
        //         println!("Unable to recv from Command Stack {}", e);
        //         None
        //     }
        // }
    }
    pub async fn send(&self, cmd: Command) {
        match self.cmds_out.send(cmd).await {
            Ok(_) => {}
            Err(e) => println!("Unable to send to Command Stack {}", e),
        }
    }
}

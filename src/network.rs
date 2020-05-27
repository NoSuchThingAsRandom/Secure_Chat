use std::fmt::Formatter;
use std::io::{Write, Read, ErrorKind};
use std::time::Duration;
use std::collections::HashMap;

use std::sync::mpsc::*;
use std::{fmt, thread, io};
use std::net::{SocketAddr, Shutdown};

use futures::io::Error;
use log::{trace, info, warn, error};


use mio::{Poll, Token, Interest};
use mio::Events;
use mio::net::{TcpListener};
use mio::net::TcpStream;
use futures::future::err;


pub(crate) const ADDR: &str = "127.0.0.1:5962";
const MAX_CLIENTS_THREAD: u8 = 10;
const SERVER: Token = Token(11);
#[derive(Clone)]
pub struct Message {
    pub(crate) data: String,
    pub sender: String,
    pub recipient: String,
}

impl Message {
    pub fn new(data: String, recipient: String, sender: String) -> Message {
        Message { data, sender, recipient }
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Message from: {}    To: {}    Contents: {}", self.sender, self.recipient, self.data)
    }
}

pub struct Client {
    pub addr: String,
    pub stream: mio::net::TcpStream,
}

impl Client {
    pub fn new(addr: String, stream: mio::net::TcpStream) -> Client {
        Client { addr, stream }
    }
}


struct ClientIo {
    clients: HashMap<Token, Client>,
    current_client_count: usize,
    poll: Poll,
    incoming_clients: Receiver<Client>,
    messages_in: Sender<Message>,
    messages_out: Receiver<Message>,
}

impl ClientIo {
    fn new(incoming_clients: Receiver<Client>, messages_in: Sender<Message>, messages_out: Receiver<Message>) -> ClientIo {
        ClientIo { clients: HashMap::new(), current_client_count: 0, poll: Poll::new().unwrap(), incoming_clients, messages_in, messages_out }
    }


    pub fn start(&mut self) -> Result<(), TryRecvError> {
        let mut events = Events::with_capacity(128);
        info!("Starting output writer loop");
        loop {
            //Check for new clients
            self.check_clients()?;

            //Send messages
            let mut messages: Vec<Message> = self.messages_out.try_iter().collect();
            for client in  self.clients.values_mut() {
                messages.retain(|msg|
                    if client.addr == msg.recipient {
                        info!("Sending new message {}", msg);
                        client.stream.write(msg.data.as_ref());
                        client.stream.flush();
                        false
                    } else {
                        true
                    });
            }
            for msg in messages {
                warn!("Failed to send {}", msg);
            }

            //Check for incoming messages
            self.poll.poll(&mut events, Some(Duration::from_millis(100)));
            for event in events.iter() {
                let mut socket = self.clients.get_mut(&event.token()).unwrap();
                info!("Received event for {}", socket.addr);
                let mut buffer = [0; 1024];
                if event.is_readable() {
                    match socket.stream.read(&mut buffer) {
                        Ok(n) => {
                            trace!("Got message of size {} from {}", n, socket.addr);
                            if n > 0 {
                                let msg = Message::new(String::from_utf8(buffer.to_vec()).expect("Invalid utf-8 received"), socket.stream.local_addr().unwrap().to_string(), socket.addr.clone());
                                info!("Received {}", msg);
                                match self.messages_in.send(msg.clone()){
                                    Ok(_) => {},
                                    Err(E) => {
                                        error!("Send message ({}) failed {}",msg,E);
                                    },
                                }
                            } else {
                                info!("Didn't read any data? {:?}", buffer.to_vec());
                                self.clients.remove(&event.token());
                                self.current_client_count-=1;
                            }
                        }
                        Err(ref err) if err.kind() != ErrorKind::WouldBlock => {
                            error!("Would block error: {}", err)
                        }
                        Err(E) => {
                            error!("Error ({}) encountered reading from {}", E, socket.addr);
                        }
                    }
                } else {
                    warn!("Non readable event {:?}", event);
                }
            }
        }
    }


    fn check_clients(&mut self) -> Result<(), TryRecvError> {
        let mut new_clients = true;
        while new_clients {
            match self.incoming_clients.try_recv() {
                Ok(mut C) => {
                    trace!("Added new client to writer {}", C.addr);
                    let token = Token(self.current_client_count);
                    self.current_client_count += 1;
                    self.poll.registry().register(&mut C.stream, token, Interest::READABLE).unwrap();
                    self.clients.insert(token, C);
                }
                Err(E) => if E == TryRecvError::Empty {
                    new_clients = false;
                } else {
                    self.shutdown();
                    return Err(E);
                },
            }
        }
        Ok(())
    }

    fn shutdown(&mut self) {
        info!("Initiating IO thread shutdown");
        for client in self.clients.values_mut() {
            client.stream.shutdown(Shutdown::Both).expect("Failed to shutdown");
        }
    }
}

pub struct Network {
    pub client_sender: Sender<Client>,
    pub connections: Vec<Client>,
}

impl Network {
    /// Function that starts IO and Listening thread
    ///
    ///
    /// # Arguments
    ///     client_addr: To send new client information to the user thread
    ///     messages_in: New messages to be sent to clients
    ///     messages_out: To send incoming messages to the user thread
    ///
    pub fn init(address:String,client_addr: Sender<String>, messages_in: Sender<Message>, messages_out: Receiver<Message>) -> Network {
        info!("Creating network handler");
        let (client_sender, client_receiver) = channel();

//Start io thread
        thread::Builder::new().name(String::from("Client IO")).spawn(move || {
            trace!("Created IO thread");
            let mut client_io = ClientIo::new(client_receiver, messages_in, messages_out);
            client_io.start();
        });

//Start listening server
        let listening_client_sender = client_sender.clone();
        thread::Builder::new().name(String::from("Listening Server")).spawn(move || {
            trace!("Created listening thread");
            Network::listen(address,client_addr, listening_client_sender);
        });

//Return network instance (With client sender for initiated connections)
        Network { client_sender, connections: Vec::new() }
    }
    /// A listening server for incoming connections that passes to IO thread
    ///
    ///
    /// # Arguments
    ///     client_addr: To send new client information to the user thread
    ///     client_sender: To send incoming clients to the IO thread
    ///
    pub fn listen(address:String,client_addr: Sender<String>, client_sender: Sender<Client>) {
        info!("Starting listening server on {}",ADDR);
        let mut poll = Poll::new().unwrap();
        let mut events = Events::with_capacity(128);
        let mut listener = mio::net::TcpListener::bind(address.parse().unwrap()).unwrap();
        poll.registry().register(&mut listener, Token(11), Interest::READABLE);
        loop {
            poll.poll(&mut events, Some(Duration::from_millis(100)));
            for _event in events.iter() {
                let (mut stream, addr) = listener.accept().unwrap();
                info!("New client connection from: {}", addr);
                client_sender.send(Client::new(addr.to_string(), stream));
                client_addr.send(addr.to_string());
            }
        }
    }
}

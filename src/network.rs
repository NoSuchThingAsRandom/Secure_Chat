use std::fmt::Formatter;
use std::io::{Write, Read, ErrorKind};
use std::time::Duration;
use std::collections::HashMap;

use std::sync::mpsc::*;
use std::{fmt, thread};
use std::net::Shutdown;

use futures::io::Error;
use log::{trace, info, warn, error};


use mio::{Poll, Token, Interest};
use mio::Events;


pub(crate) const ADDR: &str = "127.0.0.1:5962";
const MAX_CLIENTS_THREAD: u8 = 20;
const SERVER: Token = Token(11);
const MAX_MESSAGE_BYTES: u8 = 16;

#[derive(Clone, PartialEq)]
pub enum MessageOptions {
    Shutdown,
    None,
}

#[derive(Clone)]
pub struct Message {
    pub(crate) data: String,
    pub sender: String,
    pub recipient: String,
    pub options: MessageOptions,
}

impl Message {
    pub fn new(data: String, recipient: String, sender: String) -> Result<Message, Error> {
        if data.as_bytes().len() > MAX_MESSAGE_BYTES as usize {
            unimplemented!("Message data is too big!")
        }
        let message = Message { data, sender, recipient, options: MessageOptions::None };
        Ok(message)
    }
    pub fn shutdown() -> Message {
        Message {
            data: "".to_string(),
            sender: "".to_string(),
            recipient: "".to_string(),
            options: MessageOptions::Shutdown,
        }
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
    client_buffer: Vec<u8>,
}

impl Client {
    pub fn new(addr: String, stream: mio::net::TcpStream) -> Client {
        Client { addr, stream, client_buffer: Vec::new() }
    }
    pub fn close(&mut self) {
        self.stream.shutdown(Shutdown::Both);
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
        info!("Starting IO writer loop");
        loop {
            //Check for new clients
            self.check_clients()?;

            //Send messages
            let mut messages: Vec<Message> = self.messages_out.try_iter().collect();
            for client in self.clients.values_mut() {
                messages.retain(|msg|
                    if client.addr == msg.recipient {
                        info!("Sending new message {}", msg);
                        let data = msg.data.as_bytes();
                        let size = data.len() as u16;
                        let size_bytes = size.to_be_bytes();
                        client.stream.write(&size_bytes);
                        client.stream.write(data);
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
                let mut buffer: [u8; 512] = [0; 512];
                if event.is_readable() {
                    match socket.stream.read(&mut buffer) {
                        Ok(n) => {
                            trace!("Got message of size {} from {}", n, socket.addr);
                            if n > 0 {
                                socket.client_buffer.append(&mut buffer[0..n].to_vec());
                                while socket.client_buffer.len() > 2 {
                                    let cloned_buffer=socket.client_buffer.clone();
                                    let (size, buffer) = cloned_buffer.split_at(2);
                                    let data_size = u16::from_be_bytes([size[0], size[1]]);
                                    if buffer.len() > data_size as usize + 2 {
                                        let (msg_bytes, remaining_bytes) = buffer.split_at(data_size as usize).clone();

                                        let msg = Message::new(String::from_utf8(msg_bytes.to_vec()).expect("Invalid utf-8 received"), socket.stream.local_addr().unwrap().to_string(), socket.addr.clone()).unwrap();
                                        warn!("Received {}", msg);
                                        match self.messages_in.send(msg.clone()) {
                                            Ok(_) => {}
                                            Err(E) => {
                                                error!("Send message ({}) failed {}", msg, E);
                                            }
                                        }
                                        socket.client_buffer = remaining_bytes.to_vec();
                                    }
                                }
                            } else if n < 0 {
                                error!("Buffer has been filled!!!!");
                                //Buffer has been filled!
                            } else {
                                warn!("Didn't read data,closing!");
                                self.clients.remove(&event.token());
                                self.current_client_count -= 1;
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

struct NetworkWorker {
    current_connection_count: u32,
    thread_count: u32,
    client_map: HashMap<String, Sender<Message>>,
    full_client_sender: Vec<Sender<Client>>,
    slave_client_sender: Sender<Client>,
    slave_messages_out_sender: Sender<Message>,
    master_messages_in: Sender<Message>,
    master_client_address: Sender<String>,

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
    pub fn init(address: String, client_addr: Sender<String>, messages_in: Sender<Message>, mut master_messages_out: Receiver<Message>) -> Network {
        info!("Creating network handler");
        let (master_client_sender, master_client_receiver) = channel();
        thread::Builder::new().name(String::from("Listening Server")).spawn(move || {
            let mut worker = NetworkWorker::init(client_addr, messages_in);
            worker.start(address, master_client_receiver, master_messages_out);
        });

        //Start listening server


//Return network instance (With client sender for initiated connections)
        Network { client_sender: master_client_sender, connections: Vec::new() }
    }
}

impl NetworkWorker {
    fn init(master_client_address: Sender<String>, messages_in: Sender<Message>) -> NetworkWorker {
        //Start io thread
        let (mut client_sender, mut client_receiver) = channel();
        let (mut messages_out_sender, mut messages_out_receiver) = channel();
        let slave_mesages_in = messages_in.clone();
        thread::Builder::new().name(String::from("Client IO")).spawn(move || {
            trace!("Created IO thread");
            let mut client_io = ClientIo::new(client_receiver, slave_mesages_in, messages_out_receiver);
            client_io.start();
        });
        NetworkWorker { current_connection_count: 0, thread_count: 0, client_map: HashMap::new(), full_client_sender: Vec::new(), slave_client_sender: client_sender, slave_messages_out_sender: messages_out_sender, master_messages_in: messages_in, master_client_address }
    }

    fn start(&mut self, address: String, mut clients_in: Receiver<Client>, mut messages_out: Receiver<Message>) {
        info!("Starting listening server on {}", address);
        let mut poll = Poll::new().unwrap();
        let mut events = Events::with_capacity(128);
        let mut tcp_listener = mio::net::TcpListener::bind(address.parse().unwrap()).unwrap();
        poll.registry().register(&mut tcp_listener, Token(1), Interest::READABLE);

        loop {
            poll.poll(&mut events, Some(Duration::from_millis(100)));
            for event in events.iter() {
                match event.token() {
                    Token(1) => {
                        let (mut stream, addr) = tcp_listener.accept().unwrap();
                        info!("New client connection from: {}", addr);
                        self.add_client(Client::new(addr.to_string(), stream));
                    }
                    Token(_) => {
                        error!("Unknown event requested");
                        unimplemented!("Unknown event requested!");
                    }
                }
            }
            for client in clients_in.try_iter() {
                self.add_client(client);
            }
            for msg in messages_out.try_iter() {
                if msg.options == MessageOptions::Shutdown {
                    unimplemented!("Need to shutdown safely!!!");
                } else {
                    self.client_map.get_mut(msg.recipient.as_str()).unwrap().send(msg);
                }
            }
        }
    }
    fn shutdown(&mut self) {}

    fn add_client(&mut self, new_client: Client) {
        if self.current_connection_count >= MAX_CLIENTS_THREAD as u32 {
            //Update handlers
            let (new_client_sender, client_receiver) = channel();
            self.full_client_sender.push(self.slave_client_sender.clone());
            self.slave_client_sender = new_client_sender;

            let (new_messages_out_sender, messages_out_receiver) = channel();
            self.slave_messages_out_sender = new_messages_out_sender;
            let mut thread_name = String::from("Client IO - ");
            let slave_messages_in = self.master_messages_in.clone();
            thread_name.push_str(self.thread_count.to_string().as_str());
            thread::Builder::new().name(thread_name).spawn(|| {
                trace!("Created IO thread");
                let mut client_io = ClientIo::new(client_receiver, slave_messages_in, messages_out_receiver);
                client_io.start();
            });
            self.thread_count += 1;
        }
        self.current_connection_count += 1;
        self.client_map.insert(new_client.addr.to_string(), self.slave_messages_out_sender.clone());
        self.master_client_address.send(new_client.addr.to_string());
        self.slave_client_sender.send(new_client);
    }
}

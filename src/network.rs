use std::fmt::Formatter;
use std::{fmt, thread, io};
use std::net::{TcpListener, Shutdown};
use std::net::TcpStream;
use std::sync::mpsc::*;
use std::io::{Write, Read, ErrorKind};
use futures::io::Error;
use log::{trace, info, warn, error};
use std::time::Duration;

//const ADDR: String = String::from("127.0.01:5962");
const MAX_CLIENTS_THREAD: u8 = 10;

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
    pub stream: TcpStream,
}

impl Client {
    pub fn new(addr: String, stream: TcpStream) -> Client {
        Client { addr, stream }
    }
}


struct ClientIo {
    clients: Vec<Client>,
    incoming_clients: Receiver<Client>,
    messages_in: Sender<Message>,
    messages_out: Receiver<Message>,
}

impl ClientIo {
    fn new(incoming_clients: Receiver<Client>, messages_in: Sender<Message>, messages_out: Receiver<Message>) -> ClientIo {
        ClientIo { clients: Vec::new(), incoming_clients, messages_in, messages_out }
    }
    pub fn start(&mut self) -> Result<(), TryRecvError> {
        info!("Starting output writer loop");
        loop {
            //info!("IO Loop");
            //Check for new clients
            self.check_clients()?;

            //Send messages
            let mut messages: Vec<Message> = self.messages_out.try_iter().collect();

            for client in &mut self.clients {
                trace!("Checking for message {}",client.addr);
                //Check for incoming messages
                let mut buffer = Vec::new();//[0;1024];
                if client.stream.peek(&mut buffer).unwrap() > 0 {
                    match client.stream.read_to_end(&mut buffer) {
                        Ok(n) => {
                            trace!("Got message of size {}", n);
                            if n > 0 {
                                let msg = Message::new(String::from_utf8(buffer.to_vec()).expect("Invalid utf-8 received"), String::from("me"), client.addr.clone());
                                info!("Received {}", msg);
                                self.messages_in.send(msg).unwrap();
                            }
                        }
                        Err(ref err) if err.kind() != ErrorKind::WouldBlock => {
                            println!("Would block error: {}", err)
                        }
                        Err(E) => {
                            error!("Error ({}) encountered reading from {}", E, client.addr);
                        }
                    }
                }

                //Check for outgoing messages
                messages.retain(|msg|
                    if client.addr == msg.recipient {
                        trace!("Sending new message to {}", msg.sender);
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


            thread::sleep(Duration::from_secs(2));
        }
    }


    fn check_clients(&mut self) -> Result<(), TryRecvError> {
        let mut new_clients = true;
        while new_clients {
            match self.incoming_clients.try_recv() {
                Ok(C) => {
                    trace!("Added new client to writer {}", C.addr);
                    //C.stream.set_nonblocking(true);
                    self.clients.push(C)
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
        for client in &self.clients {
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
    pub fn init(client_addr: Sender<String>, messages_in: Sender<Message>, messages_out: Receiver<Message>) -> Network {
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
            Network::listen(client_addr, listening_client_sender);
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
    pub fn listen(client_addr: Sender<String>, client_sender: Sender<Client>) {
        info!("Starting listening server");
        let mut listener = TcpListener::bind("127.0.0.1:5964").unwrap();
        loop {
            let (mut stream, addr) = listener.accept().unwrap();
            info!("New client connection from: {}", addr);
            client_sender.send(Client::new(addr.to_string(), stream));
            client_addr.send(addr.to_string());
        }
    }
}

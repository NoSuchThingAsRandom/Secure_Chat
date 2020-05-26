use std::fmt::Formatter;
use std::{fmt, thread};
use std::net::{TcpListener, Shutdown};
use std::net::TcpStream;
use std::sync::mpsc::*;
use std::io::{Write, Read};
use futures::io::Error;
use log::{trace,info, warn, error};
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

fn check_clients(incoming_clients: &Receiver<Client>, clients: &mut Vec<Client>) -> Result<(), TryRecvError> {
    let mut new_clients = true;
    while new_clients {
        match incoming_clients.try_recv() {
            Ok(C) => {
                trace!("Added new client to input writer {}", C.addr);
                C.stream.set_nonblocking(true);
                clients.push(C)
            }
            Err(E) => if E == TryRecvError::Empty {
                new_clients = false;
            } else {
                info!("Closing down input writer (From clients)");
                for client in clients {
                    client.stream.shutdown(Shutdown::Both).expect("Failed to shutdown");
                }
                return Err(E);
            },
        }
    }
    Ok(())
}

struct OutputWriter {
    clients: Vec<Client>
}

impl OutputWriter {
    fn new() -> OutputWriter {
        OutputWriter { clients: Vec::new() }
    }
    pub fn start(&mut self, incoming_clients: Receiver<Client>, messages_out: Receiver<Message>) {
        info!("Starting output writer loop");
        loop {
            if check_clients(&incoming_clients, &mut self.clients).is_err() {
                return;
            }
            let mut read_messages = true;
            while read_messages {
                match messages_out.try_recv() {
                    Ok(msg) => {
                        let mut sent = false;
                        for  client in &mut self.clients {
                            if client.addr == msg.recipient {
                                trace!("Sending new message to {}", msg.sender);
                                client.stream.write(msg.data.as_ref());
                                sent = true;
                                break;
                            }
                        }
                        if !sent {
                            warn!("Client {} does not exist to send message to", msg.sender);
                        }
                    }
                    Err(E) => {
                        if E == TryRecvError::Empty {
                            read_messages = false;
                        } else {
                            for client in &mut self.clients {
                                client.stream.shutdown(Shutdown::Both).expect("Failed to shutdown");
                            }
                            info!("Closing down output writer (From messages)");
                            return;
                        }
                    }
                }
            }
        }
    }
}

struct InputReader {
    clients: Vec<Client>
}


impl InputReader {
    fn new() -> InputReader {
        InputReader { clients: Vec::new() }
    }
    pub fn start(&mut self, incoming_clients: Receiver<Client>, messages_in: Sender<Message>) {
        info!("Starting input reader loop");
        loop {
            if check_clients(&incoming_clients, &mut self.clients).is_err() {
                return;
            }
            for mut client in &mut self.clients {
                let mut buffer = String::new();
                match client.stream.read_to_string(&mut buffer) {
                    Ok(n) => {
                        if n > 0 {
                            messages_in.send(Message::new(buffer, String::from("me"), client.addr.clone())).unwrap();
                        }
                    }
                    Err(E) => {
                        error!("Error {} encountered reading from {}", E, client.addr);
                    }
                }
            }
        }
    }
}

pub struct Network {
    pub output_writer_client_sender: Sender<Client>,
    pub input_reader_client_sender: Sender<Client>,
    pub connections:Vec<Client>,
}

impl Network {
    pub fn init(client_addr:Sender<String>, messages_in: Sender<Message>, messages_out: Receiver<Message>) -> Network {
        info!("Creating network handler");
        let (output_writer_client_sender, output_writer_client_receiver) = channel();
        let (input_reader_client_sender, input_reader_client_receiver) = channel();
        thread::Builder::new().name(String::from("Output Writer")).spawn(move || {
            let mut output_writer = OutputWriter::new();
            output_writer.start(output_writer_client_receiver, messages_out);
        });
        thread::Builder::new().name(String::from("Input Reader")).spawn(move || {
            let mut input_reader = InputReader::new();
            input_reader.start(input_reader_client_receiver, messages_in);
        });
        let listen_output_writer_client_sender = output_writer_client_sender.clone();
        let listen_input_reader_client_sender = input_reader_client_sender.clone();
        thread::Builder::new().name(String::from("Listening Server")).spawn(move || {
            Network::listen(client_addr, listen_output_writer_client_sender, listen_input_reader_client_sender);
        });
        Network { output_writer_client_sender, input_reader_client_sender,connections:Vec::new() }
    }
    pub fn listen(client_addr:Sender<String>,output_writer_client_sender: Sender<Client>, input_reader_client_sender: Sender<Client>) {
        info!("Starting listening server");
        let mut listener = TcpListener::bind("127.0.0.1:5963").unwrap();
        loop {
            let (mut outgoing_stream, addr) = listener.accept().unwrap();
            info!("New client connection from: {}", addr);
            let mut incoming_stream = outgoing_stream.try_clone().unwrap();
            input_reader_client_sender.send(Client::new(addr.to_string(), incoming_stream));
            output_writer_client_sender.send(Client::new(addr.to_string(), outgoing_stream));
            client_addr.send(addr.to_string());
        }
    }
}

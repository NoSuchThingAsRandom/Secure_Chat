use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio::sync::mpsc::{Sender, channel, Receiver};


use log::{trace, info, warn, error};
use std::{fmt, time};
use std::fmt::Formatter;
use futures::future::err;


pub struct Message {
    pub(crate) data: String,
    pub from: String,
    pub to: String,
}

impl Message {
    pub fn to_me(data: String, from: String) -> Message {
        Message { data, from, to: String::from("Me") }
    }
    pub fn from_me(data: String, to: String) -> Message {
        Message { data, to, from: String::from("Me") }
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Message from: {}    To: {}    Contents: {}", self.from, self.to, self.data)
    }
}

pub struct Client {
    pub(crate) addr: String,
    pub outgoing_sender: tokio::sync::mpsc::Sender<Message>,
    pub(crate) incoming_receiver: tokio::sync::mpsc::Receiver<Message>,
}

impl Client {
    fn from_connection(addr: String, outgoing_sender: tokio::sync::mpsc::Sender<Message>, incoming_receiver: tokio::sync::mpsc::Receiver<Message>) -> Client {
        Client { addr, outgoing_sender, incoming_receiver }
    }

    ///Opens a socket connection to a given address
    pub async fn open_connection(addr: String) -> Result<Client, Box<dyn std::error::Error>> {
        info!("[Client::open_connection] Initiating connection to {}", addr);
        let mut stream = tokio::net::TcpStream::connect(&addr).await?;
        let (reader, writer) = stream.into_split();
        let (outgoing_sender, outgoing_receiver) = tokio::sync::mpsc::channel(10);
        let (incoming_sender, incoming_receiver) = tokio::sync::mpsc::channel(10);
        let mut client = Client {
            addr: addr.clone(),
            outgoing_sender,
            incoming_receiver,
        };
        trace!("[Client::open_connection] Starting input/output threads for connection");
        let addr_input = addr.clone();
        let addr_output = addr.clone();
        tokio::spawn(async move {
            dbg!("Test if this starts - input");
            Client::client_input_thread(addr_input, reader, incoming_sender).await;
        });
        println!("What the fuck");
        tokio::spawn(async move {
            dbg!("Test if this starts - output");
            Client::client_output_thread(addr_output, writer, outgoing_receiver).await;
        });
        info!("[Client::open_connection] Successfully initiated connected to {}", addr);
        //std::thread::sleep(time::Duration::from_secs(5));
        Ok(client)
    }

    /// Takes input from TCP socket and writes it to mpsc
    async fn client_input_thread(addr: String, mut reader: tokio::net::tcp::OwnedReadHalf, mut incoming_sender: tokio::sync::mpsc::Sender<Message>) {
        info!("[Socket Input] Starting for {}", addr);
        //TODO NEED TO INCREASE BUFFER SIZE
        let mut buf = [0; 1024];
        loop {
            let n = match reader.read(&mut buf).await {
                // socket closed
                Ok(n) if n == 0 => {
                    error!("[Socket Input] for {} closing", addr);
                    return;
                }
                Ok(n) => n,
                Err(e) => {
                    error!("[Socket Input] for {} failed to read error {:?}",addr, e);
                    return;
                }
            };
            let data = (&buf[0..n]).to_vec();
            let str = String::from_utf8(data);
            if str.is_ok() {
                let msg = Message::to_me(str.unwrap(), String::from(&addr));
                trace!("[Socket Input] Processed incoming message from {}, data: {}", addr, msg);
                incoming_sender.send(msg).await;
            }else{
                warn!("[Socket Input] failed to parse data ({:?}) for {}",str,addr);
            }
        }
    }
    /// Takes input from mpsc and writes it to a TCP socket
    async fn client_output_thread(addr: String, mut writer: tokio::net::tcp::OwnedWriteHalf, mut outgoing_receiver: tokio::sync::mpsc::Receiver<Message>) {
        info!("[Socket Output] starting for {}", addr);
        loop {
            let message = outgoing_receiver.recv().await;
            match message {
                Some(msg) => {
                    match writer.write(msg.data.as_bytes()).await {
                        Ok(T) => trace!("Socket Output wrote: {} for {} ", msg, addr),
                        Err(E) => error!("Socket Output failed to write: {} for {} with error: {}", msg, addr, E)
                    }
                }
                None => {
                    error!("Output writer thread for {} is closing", addr);
                    writer.forget();
                    return;
                }
            }
        }
    }
}


impl fmt::Display for Client {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.addr)
    }
}
/// Creates a new server socket
///
pub async fn listening_server(mut new_client_sender: tokio::sync::mpsc::Sender<Client>) -> Result<(), Box<dyn std::error::Error>> {
    let addr="127.0.0.1:5962".to_string();
    let mut listener = tokio::net::TcpListener::bind(addr).await?;
    trace!("[Listening Server] Listening for connection on {}", listener.local_addr().unwrap());
    loop {
        let (mut socket, addr) = listener.accept().await?;
        trace!("[Listening Server] New connection from: {}", addr);
        //TODO Establish channel size
        let (outgoing_sender, outgoing_receiver) = tokio::sync::mpsc::channel(10);
        let (incoming_sender, incoming_receiver) = tokio::sync::mpsc::channel(10);
        let client = Client::from_connection(addr.to_string(), outgoing_sender, incoming_receiver);
        let (mut reader, mut writer) = socket.into_split();
        tokio::spawn(async move {
            Client::client_input_thread(addr.to_string(), reader, incoming_sender).await;
        });
        tokio::spawn(async move {
            Client::client_output_thread(addr.to_string(), writer, outgoing_receiver).await;
        });
        new_client_sender.send(client).await;
    }
}

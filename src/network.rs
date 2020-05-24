use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio::sync::mpsc::{Sender, channel, Receiver};


use log::{trace, info, warn, error};
use std::fmt;
use std::fmt::Formatter;

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
    pub outgoing_sender: Sender<Message>,
    pub(crate) incoming_receiver: Receiver<Message>,
}

impl Client {
    fn from_connection(addr: String, outgoing_sender: Sender<Message>, incoming_receiver: Receiver<Message>) -> Client {
        Client { addr, outgoing_sender, incoming_receiver }
    }
    //noinspection RsUnresolvedReference
    pub async fn open_connection(addr: String) -> Result<Client, Box<dyn std::error::Error>> {
        info!("Initiating connection to {}", addr);
        let mut stream = TcpStream::connect(&addr).await?;
        let (reader, writer) = stream.into_split();
        let (outgoing_sender, outgoing_receiver) = channel(10);
        let (incoming_sender, incoming_receiver) = channel(10);
        let mut client = Client {
            addr: addr.clone(),
            outgoing_sender,
            incoming_receiver,
        };
        trace!("Starting input/output threads for connection");
        Client::client_input_thread(addr.clone(), reader, incoming_sender).await;
        Client::client_output_thread(addr.clone(), writer, outgoing_receiver);
        info!("Successfully initiated connected to {}", addr);
        Ok(client)
    }
    async fn client_input_thread(addr: String, mut reader: tokio::net::tcp::OwnedReadHalf, mut incoming_sender: Sender<Message>) {
        trace!("Starting input reader for {}", addr);
        tokio::spawn(async move {
            //TODO NEED TO INCREASE BUFFER SIZE
            warn!("Need to increase buffer size for receiving client data!");
            let mut buf = [0; 1024];
            loop {
                let n = match reader.read(&mut buf).await {
                    // socket closed
                    Ok(n) if n == 0 => 0,//error!("Input Socket closed {}",addr);0 },
                    Ok(n) => n,
                    Err(e) => {
                        error!("failed to read from socket; err = {:?}", e);
                        return;
                    }
                };
                if n != 0 {
                    trace!("N size: {}", n);
                    let data = (&buf[0..n]).to_vec();
                    let str = String::from_utf8(data).unwrap();
                    let msg = Message::to_me(str, String::from(&addr));
                    info!("Processed incoming message from {}, data: {}", addr, msg);
                    incoming_sender.send(msg);
                }
            }
        });
    }
    fn client_output_thread(addr: String, mut writer: tokio::net::tcp::OwnedWriteHalf, mut outgoing_receiver: Receiver<Message>) {
        trace!("Starting output writer for {}", addr);
        tokio::spawn(async move {
            let message = outgoing_receiver.recv().await;
            match message {
                Some(msg) => {
                    info!("Output writer thread writing: {}", msg);
                    let res = writer.write_all(msg.data.as_bytes()).await;
                    info!("Wrote data.. {:?}", res);
                }
                None => {
                    warn!("Output writer thread for {} recieved null?", addr);
                    //return;
                }
            }
            trace!("Writing data to {}", addr);
        });
    }
}

//noinspection RsUnresolvedReference (Won't accept tokio TcpListener)
#[tokio::main]
pub async fn listening_server(mut new_client_sender: Sender<Client>) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting the listening server");
    let mut listener = TcpListener::bind("127.0.0.1:5962").await?;
    loop {
        info!("Waiting for connection...");
        let (mut socket, addr) = listener.accept().await?;
        info!("New connection from: {:?}", addr);
        let (outgoing_sender, outgoing_receiver) = channel(10);
        let (incoming_sender, incoming_receiver) = channel(10);
        let client = Client::from_connection(addr.to_string(), outgoing_sender, incoming_receiver);
        let (mut reader, mut writer) = socket.into_split();
        Client::client_input_thread(addr.to_string(), reader, incoming_sender).await;
        Client::client_output_thread(addr.to_string(), writer, outgoing_receiver);
        new_client_sender.send(client);
    }
}

pub fn test_this() -> u32 {
    4
}

use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio::sync::mpsc::{Sender,channel,Receiver};


use log::{info, warn, error};

pub struct Message {
    pub(crate) data: String,
    from: String,
    to: String,
}

impl Message {
    fn to_me(data: String, from: String) -> Message {
        Message { data, from, to: String::from("Me") }
    }
    fn from_me(data: String, to: String) -> Message {
        Message { data, to, from: String::from("Me") }
    }
}

pub struct Client {
    pub(crate) addr: String,
    outgoing_sender: Sender<Message>,
    pub(crate) incoming_receiver: Receiver<Message>,
}

impl Client {
    fn from_connection(addr: String, outgoing_sender: Sender<Message>, incoming_receiver: Receiver<Message>) -> Client {
        Client { addr, outgoing_sender, incoming_receiver }
    }
    //noinspection RsUnresolvedReference
    async fn open_connection(addr: String) -> Result<Client, Box<dyn std::error::Error>> {
        info!("Initiating connection to {}",addr);
        let mut stream = TcpStream::connect(&addr).await;
        let (reader,writer)=stream.unwrap().into_split();
        let (outgoing_sender, outgoing_receiver) = channel(10);
        let (incoming_sender, incoming_receiver) = channel(10);
        let mut client=Client {
            addr:addr.clone(),
            outgoing_sender,
            incoming_receiver,
        };
        Client::client_input_thread(addr.clone(), reader, incoming_sender);
        Client::client_output_thread(addr,writer, outgoing_receiver);
        Ok(client)
    }
    async fn client_input_thread(addr:String, mut reader: tokio::net::tcp::OwnedReadHalf, mut incoming_sender: Sender<Message>) {
        info!("Starting input reader for {}",addr);
        tokio::spawn (async move {
            //TODO NEED TO INCREASE BUFFER SIZE
            warn!("Need to increase buffer size for receiving client data!");
            let mut buf = [0; 1024];
            loop {
                let n = match reader.read(&mut buf).await {
                    // socket closed
                    Ok(n) if n == 0 => return,
                    Ok(n) => n,
                    Err(e) => {
                        error!("failed to read from socket; err = {:?}", e);
                        return;
                    }
                };
                let data = (&buf[0..n]).to_vec();
                let str = String::from_utf8(data).unwrap();
                let msg = Message::to_me(str, String::from(&addr));
                incoming_sender.send(msg);
                info!("Processed incoming message from {}",addr);
            }
        });
    }
    async fn client_output_thread(addr:String, mut writer: tokio::net::tcp::OwnedWriteHalf, mut outgoing_receiver: Receiver<Message>) {
        info!("Starting output writer for {}",addr);
        tokio::spawn(async move {
            let message = outgoing_receiver.recv().await;
            writer.write_all(message.unwrap().data.as_bytes());
            info!("Writing data to {}", addr);
        });
    }
}

//noinspection RsUnresolvedReference (Won't accept tokio TcpListener)
#[tokio::main]
pub async fn listening_server(mut new_client_sender: Sender<Client>) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting the listening server");
    let mut listener = TcpListener::bind("127.0.0.1:8080").await?;
    loop {
        let (mut socket, addr) = listener.accept().await?;
        info!("New connection from: {:?}", addr);
        let (outgoing_sender, outgoing_receiver) = channel(10);
        let (incoming_sender, incoming_receiver) = channel(10);
        let client = Client::from_connection(addr.to_string(), outgoing_sender, incoming_receiver);
        let (mut reader, mut writer) = socket.into_split();
        Client::client_input_thread(addr.to_string(), reader, incoming_sender);
        Client::client_output_thread(addr.to_string(),writer,outgoing_receiver);
        new_client_sender.send(client);
    }
}
pub fn test_this()->u32{
    4
}

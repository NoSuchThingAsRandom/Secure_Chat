extern crate simplelog;
#[macro_use]
extern crate log;

extern crate strum;
extern crate strum_macros;

use futures::future::Join;

mod network;

use strum::IntoEnumIterator;
use strum::EnumMessage;
use strum_macros::EnumIter;
use strum_macros::EnumMessage;
use strum_macros::EnumString;
use strum_macros::AsRefStr;

use text_io::read;

use tokio::net::TcpListener;
use tokio::prelude::*;
use tokio::sync::mpsc;

use std::str::FromStr;
use std::error::Error;
use std::io::Write;
use log::LevelFilter;

use simplelog::*;
//{TermLogger, Config, TerminalMode, CombinedLogger, WriteLogger};
use std::fs::File;
use futures::executor::block_on;
use std::thread;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};

#[derive(EnumIter, EnumString, EnumMessage, Debug, AsRefStr)]
enum Commands {
    #[strum(message = "NewChat", detailed_message = "This creates a new chat")]
    NewChat,
    #[strum(message = "Listen", detailed_message = "This waits for a connection to be received")]
    Listen,
    Settings,
    Exit,

}

impl Commands {
    fn get_help_dialog() -> String {
        let mut out = String::new();
        out.push_str("Possible commands:\n");
        for command in Commands::iter() {
            out.push_str(format!("      {} - {}\n",
                                 command.get_message().unwrap_or(command.as_ref()),
                                 command.get_detailed_message().unwrap_or("No Help Message :(")).as_ref());
        }
        return out;
    }
    fn get_user_command() -> Commands {
        println!("Enter command: ");
        let input: String = read!("{}\n");
        match
        Commands::from_str(&input) {
            Ok(T) => T,
            Err(E) => {
                println!("Invalid option!\n\n{}", Commands::get_help_dialog());
                Commands::get_user_command()
            }
        }
    }
}

fn new_chat() {}

fn settings() {
    println!("Entering settings");
    println!("Quitting settings");
}

fn check_messages(clients:&Vec<Client>){
    for client in clients{
        let mut msg =client.incoming_receiver.recv();
        while msg.is_ok() {
            println!("New message from: {}\n{}",client.addr,msg.unwrap().data);
             msg=client.incoming_receiver.recv();
        }
    }

}

fn input_loop(incoming_reciever:Receiver<Client>) {
    let mut clients:Vec<Client>=Vec::new();
    info!("Started input loop");
    loop {
        match incoming_reciever.try_recv(){
            Ok(C) => {
                info!("Added new client");
                clients.push(C)},
            Err(E) => {},
        }
        check_messages(&clients);
        match Commands::get_user_command() {
            Commands::NewChat => {
                new_chat()
            }
            Commands::Settings => {
                settings()
            }
            Commands::Exit => {
                println!("Goodbye!");
                return;
            }
            Commands::Listen => {}
        }
    }
}

fn main() {
    CombinedLogger::init(
        vec![
            TermLogger::new(LevelFilter::Info, Config::default(), TerminalMode::Stdout),
            WriteLogger::new(LevelFilter::Info, Config::default(), File::create("My_log.log").unwrap()),
        ]
    ).unwrap();
    info!("Initiating Setup!");
    let (new_client_sender, new_client_receiver)=channel();
    thread::spawn(|| {
        info!("Starting input loop");
        input_loop(new_client_receiver);
    });
    test(new_client_sender);
}
struct Message{
    data:String,
    from:String,
    to:String,
}
impl Message{
    fn to_me(data:String,from:String)->Message{
        Message{data,from,to: String::from("Me") }
    }
    fn from_me(data:String,to:String)->Message{
        Message{data,to,from: String::from("Me") }
    }
}
struct Client{
    addr:String,
    outgoing_sender:Sender<Message>,
    incoming_receiver:Receiver<Message>,
}
impl Client{
    fn new(addr:String,outgoing_sender:Sender<Message>,incoming_receiver:Receiver<Message>)->Client{
        Client{addr,outgoing_sender,incoming_receiver}
    }
}
#[tokio::main]
async fn test(new_client_sender:Sender<Client>) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting the listening server");
    let mut listener = TcpListener::bind("127.0.0.1:8080").await?;
    loop {
        let (mut socket, addr) = listener.accept().await?;
        info!("New connection from: {:?}", addr);
        let  (outgoing_sender,outgoing_receiver)=channel();
        let  (incoming_sender,incoming_receiver)=channel();
        let client=Client::new(addr.to_string(),outgoing_sender,incoming_receiver);
        new_client_sender.send(client);
        let (mut reader,mut writer)=socket.into_split();
        tokio::spawn(async move {
            let mut buf = [0; 1024];

            // In a loop, read data from the socket and write the data back.
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
                //info!("New message from: {}\n  {}", addr, str);
                let msg=Message::to_me(str,addr.to_string());
                incoming_sender.send(msg);//.await.unwrap();
                info!("Processed incoming message");
            }
        });
        tokio::spawn(async move {
            let message=outgoing_receiver.recv().unwrap();
            writer.write_all(message.data.as_bytes());
            info!("Writing data to {}",addr);
        });

    }
}

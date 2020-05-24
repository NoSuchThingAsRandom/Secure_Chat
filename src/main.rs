extern crate simplelog;
#[macro_use]
extern crate log;

use secure_chat_lib;

use log::LevelFilter;

use simplelog::*;
use std::fs::File;

use std::{thread, time};
use tokio::sync::mpsc::channel;
use secure_chat_lib::update_clients;
use secure_chat_lib::network::{Client, Message};
use std::error::Error;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::prelude::*;
//use tokio::io::util::async_read_ext::AsyncReadExt;
#[tokio::main]
async fn test() {
    let messages = vec!("Hello", "How are you?");
    let addr = String::from("127.0.0.1:5962");
    info!("Attempting connection...");
    let mut client = match Client::open_connection(addr.clone()).await {
        Ok(T) => T,
        Err(E) => {
            error!("Failed to connect to server! {}", E);
            return;
        }
    };
    info!("Connected to server, sending messagess");
    for msg in messages {
        let sub_message = Message::from_me(String::from(msg), addr.clone());
        trace!("Sending message {} to writer", sub_message);
        client.outgoing_sender.send(sub_message).await;
        std::thread::sleep(time::Duration::from_secs(2));
    }
    info!("Send messages");
    /*    thread::spawn(move || {
            let mut clients: Vec<Client> = Vec::new();

            secure_chat_lib::network::listening_server(new_client_sender);
            info!("Updating clients");
            update_clients(&mut new_client_receiver, &mut clients);
            info!("Client size {}", clients.len());
        });*/
}

#[tokio::main]
async fn test_stupid() {
    let (mut outgoing_sender, mut outgoing_receiver) = channel(10);
    outgoing_sender.send("A").await;
    outgoing_sender.send("B").await;
    tokio::spawn(async move {});
    tokio::spawn(async move {
        println!("{}", outgoing_receiver.recv().await.unwrap());
        println!("{}", outgoing_receiver.recv().await.unwrap());
    });
}


#[tokio::main]
async fn simple_reader() ->Result<(),Box<dyn std::error::Error>>{
    println!("What da fujhc");
    tokio::spawn(async move {
        println!("What da fujhc X2");
        info!("Listening");
        let mut listener = TcpListener::bind("127.0.0.1:5962").await;
        let mut asda=listener.unwrap();
        loop {
            info!("Waiting for connection...");

            info!("Connecterd to clioen");

            let (mut socket, addr) = asda.accept().await.unwrap();
            info!("Connecterd to clioeashjfsdhjfvbsdjhfvgbdshjfbsdhjdfbsdn");
            let (mut reader, mut writer) = socket.into_split();
            let mut buffer = [0; 1024];
            let n = reader.read(&mut buffer).await.unwrap();
            let data = (&buffer[0..n]).to_vec();
            let str = String::from_utf8(data).unwrap();
            println!("Read {} bytes", n);
            println!("Msg was: {}", str);
        }
    });

    Ok(())
}
#[tokio::main]
async fn simple_writer(){
    println!("Sup");
    let msg="Hello";
    let mut stream = TcpStream::connect("127.0.0.1:5962").await;
    info!("Connneted");
    let (reader,mut writer) = stream.unwrap().into_split();
    writer.write_all(msg.as_bytes()).await;
    info!("Wrote all data");
}

pub fn main() {



    let mut config = ConfigBuilder::new();
    config.set_location_level(LevelFilter::Error);
    config.set_thread_level(LevelFilter::Error);
    config.set_time_level(LevelFilter::Error);
    CombinedLogger::init(
        vec![
            TermLogger::new(LevelFilter::Trace, config.build(), TerminalMode::Stdout),
            WriteLogger::new(LevelFilter::Trace, config.build(), File::create("Logs/master.log").unwrap()),
        ]
    ).unwrap();

    simple_reader();
    println!("Starting writing");
    simple_writer();
    println!("Finsihyed writing");
    /*
    info!("Initiating Setup!");
    let (new_client_sender, new_client_receiver) = channel(100);
    thread::spawn(|| {
        secure_chat_lib::network::listening_server(new_client_sender);

    });
    thread::spawn(|| {
        info!("Starting user thread..");

        secure_chat_lib::input_loop(new_client_receiver);
    });

    info!("Starting client connection thread");
    std::thread::sleep(time::Duration::from_secs(2));
    info!("Waiting...");
    test();*/

}

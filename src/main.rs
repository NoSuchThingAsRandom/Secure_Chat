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

async fn test() {
    let messages = vec!("Hello\n", "How are you?\n");
    let addr = String::from("127.0.0.1:5962");
    info!("Attempting connection to {}",addr);
    let mut client = match Client::open_connection(addr.clone()).await {
        Ok(T) => T,
        Err(E) => {
            error!("Failed to connect to server! {}", E);
            return;
        }
    };
    info!("Connected to server");
    std::thread::sleep(time::Duration::from_secs(5));
    info!("Starting message sending");
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
pub async fn main() {
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

    info!("Initiating Setup!");
    let (new_client_sender, new_client_receiver) = tokio::sync::mpsc::channel(10);
    tokio::spawn(async move {
        secure_chat_lib::network::listening_server(new_client_sender).await;
    });
    tokio::spawn(async move {
        info!("Starting user thread..");
        secure_chat_lib::input_loop(new_client_receiver);
    });
    loop{

    }

/*    std::thread::sleep(time::Duration::from_secs(5));
    info!("Starting client connection thread");
    test().await;*/
}

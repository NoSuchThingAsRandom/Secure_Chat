extern crate simplelog;
#[macro_use]
extern crate log;

use secure_chat_lib;
use network::Client;
use network::Message;
use log::LevelFilter;

use simplelog::*;
use std::fs::{File, read};

use std::{thread, time};

use secure_chat_lib::{update_clients, network};

use std::error::Error;

use std::sync::mpsc::{channel, TryRecvError};
use std::net::TcpStream;
use text_io::read;
//use tokio::io::util::async_read_ext::AsyncReadExt;
/*
async fn test() {
    let messages = vec!("Hello\n", "How are you?\n");
    let addr = String::from("127.0.0.1:5962");
    info!("Attempting connection to {}", addr);
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
    *//*    thread::spawn(move || {
            let mut clients: Vec<Client> = Vec::new();

            secure_chat_lib::network::listening_server(new_client_sender);
            info!("Updating clients");
            update_clients(&mut new_client_receiver, &mut clients);
            info!("Client size {}", clients.len());
        });*//*
}*/


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

    info!("Initiating Setup!");
    let (user_client_sender, user_client_receiver) = channel();
    let (messages_in_sender, messages_in_receiver) = channel();
    let (messages_out_sender, messages_out_receiver) = channel();
    let network_struct = network::Network::init(user_client_sender, messages_in_sender, messages_out_receiver);
    let mut clients: Vec<String> = Vec::new();
    loop {
        println!("Etner command:");
        let command: String = read!("{}\n");
        match command.as_str() {
            "Connect" => {
                println!("Enter hostname:");
                let hostname: String = read!("{}\n");
                match TcpStream::connect(hostname) {
                    Ok(stream) => {
                        let addr = stream.peer_addr().unwrap();
                        info!("Created new connection to {:?}", stream.peer_addr());

                        network_struct.client_sender.send(Client::new(addr.to_string(), stream));

                        clients.push(addr.to_string());
                    }
                    Err(E) => {
                        error!("Failed to create new client {}", E);
                    }
                }
            }
            "Message" => {
                println!("Etner the client name: ");
                let name: String = read!("{}\n");
                println!("Etner the message: ");

                let data: String = read!("{}\n");
                if clients.contains(&name) {
                    messages_out_sender.send(Message::new(data, name, String::from("ME")));
                } else {
                    println!("Address doesn't exist");
                }
            }
            "List" => {
                println!("Current clients: ");
                for client in &clients {
                    println!("    {}", client);
                }
            }
            "Update" => {
                let mut read_msg = true;
                while read_msg {
                    match messages_in_receiver.try_recv() {
                        Ok(msg) => {
                            println!("Received new message {}", msg);
                        }
                        Err(E) => {
                            if E == TryRecvError::Empty {
                                read_msg = false;
                            } else {
                                error!("Failed to read message {}", E);
                                read_msg = false;
                            }
                        }
                    }
                }
            }
            _ => { println!("Unknown command") }
        }
    }

    /*    std::thread::sleep(time::Duration::from_secs(5));
        info!("Starting client connection thread");
        test().await;*/
}

extern crate strum;
extern crate strum_macros;


use strum::IntoEnumIterator;
use strum::EnumMessage;
use strum_macros::EnumIter;
use strum_macros::EnumMessage;
use strum_macros::EnumString;
use strum_macros::AsRefStr;

use log::{trace,info, warn, error};
use text_io::read;


use std::str::FromStr;
use std::error::Error;
use std::io::Write;
use tokio::sync::mpsc::Receiver;

use network::Client;
use network::Message;
use futures::executor::block_on;

pub mod network;


#[derive(EnumIter, EnumString, EnumMessage, Debug, AsRefStr)]
enum Commands {
    #[strum(message = "NewChat", detailed_message = "This creates a new chat")]
    NewChat,
    #[strum(message = "Listen", detailed_message = "This waits for a connection to be received")]
    Listen,
    #[strum(message = "Connections", detailed_message = "This displays all current connections")]
    Connections,
    #[strum(message = "Message", detailed_message = "This sends a message")]
    Message,
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
        println!("\n\nEnter command: ");
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

fn new_chat() -> Result<Client, Box<dyn Error>> {
    println!("Enter the hostname: ");
    let hostname: String = read!("{}\n");
    block_on(Client::open_connection(hostname))

}

fn settings() {
    println!("Entering settings");
    println!("Quitting settings");
}

pub fn get_messages(clients: &mut Vec<Client>) -> Vec<Message> {
    let mut messages = Vec::new();
    for client in clients {
        let mut msg = client.incoming_receiver.try_recv();
        while msg.is_ok() {
            messages.push(msg.unwrap());
            msg = client.incoming_receiver.try_recv();
        }
    }
    messages
}

pub fn input_loop(mut incoming_receiver: tokio::sync::mpsc::Receiver<Client>) {
    let mut clients: Vec<Client> = Vec::new();
    info!("Started input loop");
    loop {
        update_clients(&mut incoming_receiver, &mut clients);
        check_messages(&mut clients);
        match Commands::get_user_command() {
            Commands::NewChat => {
                match new_chat(){
                    Ok(C) => {clients.push(C);println!("Connected to client");info!("Connected to new client");},
                    Err(E) => {
                        println!("Failed to connect to hostname");
                        error!("Failed to connect to hostname {}",E);
                    },
                }
            }
            Commands::Settings => {
                settings()
            }
            Commands::Exit => {
                println!("Goodbye!");
                return;
            }
            Commands::Listen => {}
            Commands::Connections => {
                println!("Current connections: ");
                for client in &clients{
                    println!("      {}",&client);
                }
            }
            Commands::Message => {
                println!("Enter the name of client to message: ");
                let name: String = read!("{}\n");
                let mut sent =false;
                for client in &mut clients{
                    if client.addr==name {
                        println!("Enter the message to send: ");
                        let msg:String=read!("{}\n");
                        trace!("Making message request to {}",&client);
                        block_on(client.outgoing_sender.send(Message::from_me(msg, String::from(&client.addr))));
                        println!("\n Sent message");
                        sent=true;
                        break
                    }
                }
                if !sent{
                    println!("Failed to find recipient");
                }
            }
        }
    }
}


pub fn check_messages(clients: &mut Vec<Client>) {
    info!("Updating messages");
    for msg in get_messages(clients) {
        println!("New message from: {}\n{}", msg.from, msg.data);
    }
}

pub fn update_clients(incoming_receiver: &mut tokio::sync::mpsc::Receiver<Client>, clients: &mut Vec<Client>) {
    trace!("Checking for new clients");
    match incoming_receiver.try_recv() {
        Ok(C) => {
            info!("Added new client");
            clients.push(C)
        }
        Err(E) => { info!("No clients available\n{}", E) }
    }
}


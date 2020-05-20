extern crate strum;
extern crate strum_macros;


use strum::IntoEnumIterator;
use strum::EnumMessage;
use strum_macros::EnumIter;
use strum_macros::EnumMessage;
use strum_macros::EnumString;
use strum_macros::AsRefStr;

use log::{info,warn,error};
use text_io::read;



use std::str::FromStr;
use std::error::Error;
use std::io::Write;
use tokio::sync::mpsc::Receiver;

use network::Client;
use network::Message;
pub mod network;


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

fn new_chat() {
    println!("Enter the hostname: ");
    let hostname:String =read!("{}\n");
}

fn settings() {
    println!("Entering settings");
    println!("Quitting settings");
}

fn check_messages(clients:&mut Vec<Client>){
    for  client in clients{
        let mut msg =client.incoming_receiver.try_recv();
        while msg.is_ok() {
            println!("New message from: {}\n{}",client.addr,msg.unwrap().data);
            msg=client.incoming_receiver.try_recv();
        }
    }

}

pub fn input_loop(mut incoming_receiver: Receiver<Client>) {
    let mut clients:Vec<Client>=Vec::new();
    info!("Started input loop");
    loop {
        match incoming_receiver.try_recv(){
            Ok(C) => {
                info!("Added new client");
                clients.push(C)},
            Err(E) => {},
        }
        check_messages(&mut clients);
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

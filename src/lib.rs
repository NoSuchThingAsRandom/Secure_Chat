extern crate strum;
extern crate strum_macros;
use crate::network::{Message, Client, Network, ADDR};

use strum::IntoEnumIterator;
use strum::EnumMessage;
use strum_macros::EnumIter;
use strum_macros::EnumMessage;
use strum_macros::EnumString;
use strum_macros::AsRefStr;

use log::{trace, info, warn, error};
use text_io::read;

use mio::net::TcpStream;


use std::str::FromStr;
use std::error::Error;
use std::io::Write;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::{fmt, thread};
use std::fmt::Formatter;
use std::net::SocketAddr;
use std::time::Duration;

pub mod network;


#[derive(EnumIter, EnumString, EnumMessage, Debug, AsRefStr)]
enum Commands {
    #[strum(message = "Connect", detailed_message = "This creates a new chat")]
    Connect,
    #[strum(message = "Update", detailed_message = "This retrieves any new messages and connections")]
    Update,
    #[strum(message = "List", detailed_message = "This displays all current connections")]
    List,
    #[strum(message = "Message", detailed_message = "This sends a message")]
    Message,
    Test,
    #[strum(message = "Exit", detailed_message = "This exits the program")]
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


struct ClientUser {
    addr: String,
    messages: Vec<Message>,
    nickname: String,
}

impl fmt::Display for ClientUser {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Client with address {}, and nickname {}", self.addr, self.nickname)
    }
}

pub struct InputLoop {
    user_client_receiver: Receiver<String>,
    messages_in_receiver: Receiver<Message>,
    messages_out_sender: Sender<Message>,
    clients: Vec<ClientUser>,
    network_struct: Network,
}

impl InputLoop {
    pub fn new() -> InputLoop {
        let (user_client_sender, user_client_receiver) = channel();
        let (messages_in_sender, messages_in_receiver) = channel();
        let (messages_out_sender, messages_out_receiver) = channel();
        let network_struct = network::Network::init(user_client_sender, messages_in_sender, messages_out_receiver);

        InputLoop {
            user_client_receiver,
            messages_in_receiver,
            messages_out_sender,
            clients: Vec::new(),
            network_struct,
        }
    }
    ///The main user input loop
    /// Responds to user input
    pub fn start(&mut self) {
        info!("Started user input loop");
        loop {
            match Commands::get_user_command() {
                Commands::Connect => {
                    println!("Enter hostname:");
                    let host_string: String = read!("{}\n");
                    match self.connect(host_string) {
                        Some(C) => {
                            println!("Connected to client {}", C);
                            self.clients.push(C);
                        }
                        None => {
                            println!("Failed to connect to client");
                        }
                    }
                }
                Commands::Message => {
                    println!("Enter the name of client to message: ");
                    let name: String = read!("{}\n");
                    println!("Enter the message to send: ");
                    let msg: String = read!("{}\n");

                    if self.send_messsage(name, msg) {
                        println!("Sent message");
                    } else {
                        println!("Failed to find recipient");
                    }
                }
                Commands::Update => {
                    self.update_clients();
                    self.check_messages();
                }
                Commands::List => {
                    println!("Current connections: ");
                    for client in &self.clients {
                        println!("      {}", &client);
                    }
                }
                Commands::Exit => {
                    println!("Goodbye!");
                    return;
                }
                Commands::Test => {
                    self.test();
                }
            }
        }
    }
    fn check_messages(&mut self) {
        info!("Updating messages");
        for msg in self.messages_in_receiver.try_iter() {
            for mut client in &mut self.clients {
                if client.addr == msg.sender {
                    println!("{}", msg);
                    client.messages.push(msg.clone());
                }
            }
        }
    }

    fn update_clients(&mut self) {
        trace!("Checking for new clients");
        for client in self.user_client_receiver.try_iter() {
            println!("New client from {}", client);

            self.clients.push(ClientUser {
                addr: client,
                messages: vec![],
                nickname: "".to_string(),
            })
        }
    }

    fn connect(&mut self, host_string: String) -> Option<ClientUser> {
        //Check hostname
        let hostname = host_string.parse();
        if hostname.is_err() {
            println!("Invalid address!\nTry again");
            return None;
        }

        //Initiate Connection
        match mio::net::TcpStream::connect(hostname.unwrap()) {
            Ok(stream) => {
                let addr = stream.peer_addr().unwrap();
                let local_addr = stream.local_addr().unwrap();
                info!("Created new connection to {:?}", stream.peer_addr());
                if self.network_struct.client_sender.send(Client::new(addr.to_string(), stream)).is_err() {
                    panic!("The IO thread has been closed!");
                }
                Some(ClientUser {
                    addr: addr.to_string(),
                    messages: vec![],
                    nickname: local_addr.to_string(),
                })
            }
            Err(E) => {
                error!("Failed to create new client {}", E);
                None
            }
        }
    }

    fn send_messsage(&mut self, client_name: String, msg_data: String) -> bool {
        for client in &mut self.clients {
            if client.addr == client_name {
                trace!("Making message request to {}", &client);
                let msg = Message::new(msg_data, client_name, String::from("ME"));
                self.messages_out_sender.send(msg);
                return true;
            }
        }
        false
    }
    fn test(&mut self) {
        let messages = ["Hello"; 50];//vec!("Hello\n", "How are you?\n");
        let mut address=Vec::new();
        for _ in 0..10 {
            let client = self.connect((&ADDR.to_string()).to_string()).expect("Failed to open testing client");
            let name=client.nickname.clone();
            address.push(name);
            self.clients.push(client);
        }
        info!("Created clients");
        self.update_clients();

        for msg in messages.iter() {
            for c in &address {
                self.send_messsage((&c).to_string(), (msg.to_string()).to_string());
                thread::sleep(Duration::from_millis(10));
            }
            self.check_messages();
        }
        info!("Finished");
    }
}












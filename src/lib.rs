extern crate strum;
extern crate strum_macros;

use crate::network::{Message, Client, Network};

use strum::IntoEnumIterator;
use strum::EnumMessage;
use strum_macros::EnumIter;
use strum_macros::EnumMessage;
use strum_macros::EnumString;
use strum_macros::AsRefStr;

use log::{trace, info, error};
use text_io::read;




use std::str::FromStr;

use std::sync::mpsc::{Receiver, Sender, channel};
use std::{fmt, thread};
use std::fmt::Formatter;

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
    pub fn new(address: String) -> InputLoop {
        let (user_client_sender, user_client_receiver) = channel();
        let (messages_in_sender, messages_in_receiver) = channel();
        let (messages_out_sender, messages_out_receiver) = channel();
        let network_struct = network::Network::init(address, user_client_sender, messages_in_sender, messages_out_receiver);

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

                    if self.send_message(name, msg) {
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
                    self.test_multi_server_multi_client();
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
    fn check_messages_bench(&mut self) -> Vec<i64> {
        info!("Updating messages");
        let mut differences = Vec::new();
        for msg in self.messages_in_receiver.try_iter() {
            for mut client in &mut self.clients {
                if client.addr == msg.sender {
                    let time: Result<i64, <i64 as FromStr>::Err> = msg.data.parse();
                    match time {
                        Ok(T) => {
                            let rec_time: i64 = chrono::Utc::now().timestamp_millis();
                            println!("Time {} dif {}", rec_time, T);
                            println!("Received {}", rec_time - T);
                            differences.push(rec_time - T);
                        }
                        Err(_) => {
                            for fuck in msg.data.split_whitespace() {
                                let time: Result<i64, <i64 as FromStr>::Err> = fuck[0..fuck.len()].parse();
                                if time.is_ok() {
                                    let current = chrono::Utc::now().timestamp_millis();
                                    let receievd = time.unwrap();
                                    //println!("CUrrent {} Time Sent{}",current,receievd);
                                    let different = current - receievd;
                                    differences.push(different);
                                }
                            }
                        }
                    }
                }
            }
        }
        differences
    }

    fn update_clients(&mut self) {
        trace!("Checking for new clients");
        for client in self.user_client_receiver.try_iter() {
            //println!("New client from {}", client);

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

    fn send_message(&mut self, client_name: String, msg_data: String) -> bool {
        println!("Attempting to send message to {}", client_name);
        for client in &mut self.clients {
            if client.addr == client_name {
                trace!("Making message request to {}", &client);
                return match Message::new(msg_data, client_name, String::from("ME")) {
                    Ok(msg) => {
                        self.messages_out_sender.send(msg);
                        true
                    },
                    Err(E) => {
                        println!("Message data is too big!");
                        false
                    }
                }
            }
        }
        false
    }

    fn shutdown(&mut self) {
        self.messages_out_sender.send(Message::shutdown());
    }
    pub fn test_multi_server_multi_client(&mut self) {
        info!("Starting multi client multi server test");
        // Test consts
        const NUM_THREADS: u32 = 10;
        const NUM_INSTANCES: u32 = 120;
        let instance_per_thread = (NUM_INSTANCES / NUM_THREADS) as usize;
        const NUM_MESSAGES: u32 = 500;
        const PORT: u32 = 50000;

        let mut addresses = Vec::new();
        let mut current_thread_state: Vec<InputLoop> = Vec::new();
        let mut all_states: Vec<Vec<InputLoop>> = Vec::new();
        println!("Starting instances");
        //Start instances
        for instances_index in 0..NUM_INSTANCES {
            let mut host = String::from("127.0.0.1:");
            host.push_str((PORT + instances_index).to_string().as_str());
            let mut state = InputLoop::new(host.clone());
            addresses.push(host);
            if current_thread_state.len() == instance_per_thread {
                all_states.push(current_thread_state);
                current_thread_state = Vec::new();
            }
            current_thread_state.push(state);
        }
        all_states.push(current_thread_state);
        info!("Created instances");
        //Start threads
        let mut threads = Vec::new();
        for mut state in all_states {
            let address_copy: Vec<String> = addresses.clone();
            threads.push(thread::spawn(move || {

                //Connect to other clients
                for sub_state in &mut state {
                    for address in &address_copy {
                        match sub_state.connect(String::from(address)) {
                            Some(C) => {
                                sub_state.clients.push(C);
                            }
                            None => {
                                println!("Failed to connect to client");
                            }
                        }
                    }
                }
                println!("Created connections");
                thread::sleep(Duration::from_secs(2));
                for sub_state in &mut state {
                    sub_state.update_clients();
                }


                //Send messages
                let mut differences: Vec<i64> = Vec::new();
                for msg_index in 0..NUM_MESSAGES {
                    for sub_state in &mut state {
                        for address in &address_copy {
                            let mut msg = chrono::Utc::now().timestamp_millis().to_string();
                            msg.push(' ');
                            sub_state.send_message(address.clone(), msg);
                        }
                        differences.append(&mut sub_state.check_messages_bench());
                    }
                }


                //Calculate average time difference
                println!("Sent messages");
                thread::sleep(Duration::from_secs(10));
                for sub_state in &mut state {
                    for timestamp in sub_state.check_messages_bench().iter() {
                        differences.push(timestamp - 10000);
                    }
                }
                let mut total: i64 = 0;
                for dif in &differences {
                    total = total + *dif as i64;
                }
                thread::sleep(Duration::from_secs(1));
                if total > 0 {
                    total = total / differences.len() as i64;
                    println!("Mean difference in millis (maybe?) {}", total);
                } else {
                    println!("Time fucked up? {}", total);
                }
                info!("     Sent messages");
                total
            }));
        }
        let size = threads.len();
        let mut total = 0;
        for thread in threads {
            let score = thread.join().unwrap();
            if score > 0 {
                total += score;
            }
        }
        println!("\n\nTotal Average {}", total / size as i64);


        self.update_clients();
        info!("Finished");
    }

    /*pub fn test_single_server_multi_client(&mut self) {
        info!("Starting multi client multi server test");
        // Test consts
        const NUM_THREADS: u32 = 10;
        const NUM_INSTANCES: u32 = 120;
        let instance_per_thread = (NUM_INSTANCES / NUM_THREADS) as usize;
        const NUM_MESSAGES: u32 = 500;
        const PORT: u32 = 50000;

        let mut addresses = Vec::new();
        let mut current_thread_state: Vec<InputLoop> = Vec::new();
        let mut all_states: Vec<Vec<InputLoop>> = Vec::new();
        println!("Starting instances");
        //Start instances
        for instances_index in 0..NUM_INSTANCES {
            let mut host = String::from("127.0.0.1:");
            host.push_str((PORT + instances_index + 1).to_string().as_str());
            let state = InputLoop::new(host.clone());
            addresses.push(host);
            if current_thread_state.len() == instance_per_thread {
                all_states.push(current_thread_state);
                current_thread_state = Vec::new();
            }
            current_thread_state.push(state);
        }
        all_states.push(current_thread_state);

        //Start Server Thread
        thread::spawn(move || {
            let mut host = String::from("127.0.0.1:");
            host.push_str((PORT).to_string().as_str());
            let mut state = InputLoop::new(host);
            while state.clients.len() != NUM_INSTANCES as usize {
                state.update_clients();
            }
            //Calculate average time difference
            println!("Sent messages");


            for timestamp in sub_state.check_messages_bench().iter() {
                differences.push(timestamp - 10000);
            }
            let mut total: i64 = 0;
            for dif in &differences {
                total = total + *dif as i64;
            }
            thread::sleep(Duration::from_secs(1));
            if total > 0 {
                total = total / differences.len() as i64;
                println!("Mean difference in millis (maybe?) {}", total);
            } else {
                println!("Time fucked up? {}", total);
            }
            info!("     Sent messages");

            let size = threads.len();
            let mut total = 0;

            println!("\n\nTotal Average {}", total / size as i64);
        });

        thread::sleep(Duration::from_secs(5));
        info!("Created instances");
        //Start client threads
        let mut threads = Vec::new();
        for mut state in all_states {
            let address_copy: Vec<String> = addresses.clone();
            threads.push(thread::spawn(move || {
                let mut host = String::from("127.0.0.1:");
                host.push_str((PORT).to_string().as_str());
                //Connect to server
                for sub_state in &mut state {
                    match sub_state.connect(host.clone()) {
                        Some(C) => {
                            sub_state.clients.push(C);
                        }
                        None => {
                            println!("Failed to connect to client");
                        }
                    }
                }
                println!("Created connections");
                thread::sleep(Duration::from_secs(5));

                //Send messages
                for msg_index in 0..NUM_MESSAGES {
                    for sub_state in &mut state {
                        let mut msg = chrono::Utc::now().timestamp_millis().to_string();
                        msg.push(' ');
                        sub_state.send_message(host.clone(), msg);
                    }
                }
                state.get_mut(0).
            }));
        }


        //Stop client threads
        for thread in threads {
            thread.join().unwrap();
        }

        self.update_clients();
        info!("Finished");
    }*/

    pub fn fish(&mut self) {
        thread::sleep(Duration::from_secs(1));

        let client = self.connect(String::from("127.0.0.1:49999")).unwrap();
        self.clients.push(client);

        for value in String::from("ABCDEF").split("") {
            println!("{}", value);
            println!("{}", self.send_message(String::from("127.0.0.1:49999"), value.to_string()));
        }

        thread::sleep(Duration::from_secs(5));
        thread::sleep(Duration::from_secs(1));
        self.shutdown();
        println!("Done");
    }
}












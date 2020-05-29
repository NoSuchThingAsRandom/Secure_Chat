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

    pub fn shutdown(&mut self) {
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
        let lorem=
            ["Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Tempus iaculis urna id volutpat lacus laoreet non. In egestas erat imperdiet sed euismod nisi porta lorem mollis. Praesent elementum facilisis leo vel. Arcu bibendum at varius vel pharetra vel turpis nunc. Aliquet nec ullamcorper sit amet risus nullam eget. Quam quisque id diam vel quam elementum pulvinar etiam. Ullamcorper sit amet risus nullam eget. Sodales neque sodales ut etiam sit. Nunc mi ipsum faucibus vitae aliquet. Nunc sed augue lacus viverra. Neque volutpat ac tincidunt vitae semper quis lectus. Tortor at auctor urna nunc id cursus metus aliquam eleifend. Ligula ullamcorper malesuada proin libero nunc consequat interdum. Et netus et malesuada fames ac turpis egestas integer eget. Nulla aliquet enim tortor at auctor urna nunc id cursus.

Diam ut venenatis tellus in metus vulputate eu scelerisque felis. Sodales ut eu sem integer vitae justo eget. Arcu non sodales neque sodales ut etiam sit amet. Neque gravida in fermentum et. Metus aliquam eleifend mi in nulla posuere sollicitudin. Sed viverra ipsum nunc aliquet bibendum enim facilisis. Ultrices vitae auctor eu augue ut lectus arcu bibendum. Vestibulum morbi blandit cursus risus. Diam phasellus vestibulum lorem sed risus ultricies tristique nulla. Nullam eget felis eget nunc lobortis. Consequat ac felis donec et odio pellentesque diam volutpat commodo. At consectetur lorem donec massa. Rhoncus urna neque viverra justo. Quis viverra nibh cras pulvinar mattis nunc sed. Urna id volutpat lacus laoreet non curabitur.

Nec sagittis aliquam malesuada bibendum arcu vitae elementum curabitur. Lacus vel facilisis volutpat est velit egestas dui id. Quis risus sed vulputate odio ut enim blandit. Nisl suscipit adipiscing bibendum est ultricies integer quis auctor. Gravida arcu ac tortor dignissim convallis aenean et tortor at. Nunc sed augue lacus viverra vitae. Sed faucibus turpis in eu mi. Sodales ut eu sem integer. Ac felis donec et odio pellentesque diam volutpat. Erat pellentesque adipiscing commodo elit at imperdiet. Ullamcorper malesuada proin libero nunc consequat interdum varius. Eros in cursus turpis massa. Sed arcu non odio euismod lacinia at quis risus. Tincidunt lobortis feugiat vivamus at augue eget. Amet cursus sit amet dictum sit amet justo. Elit at imperdiet dui accumsan. Tellus at urna condimentum mattis pellentesque id. Sit amet nulla facilisi morbi tempus. Velit egestas dui id ornare arcu odio ut sem. Augue neque gravida in fermentum et sollicitudin ac.

Scelerisque in dictum non consectetur a erat nam at. Viverra tellus in hac habitasse platea dictumst. Fringilla est ullamcorper eget nulla facilisi etiam dignissim. Aenean et tortor at risus. Pulvinar mattis nunc sed blandit libero volutpat. Aliquam malesuada bibendum arcu vitae elementum curabitur vitae. Risus sed vulputate odio ut enim. Ac auctor augue mauris augue. Dolor sit amet consectetur adipiscing elit duis tristique sollicitudin. Augue mauris augue neque gravida in. Auctor elit sed vulputate mi sit amet mauris commodo quis. Bibendum enim facilisis gravida neque convallis a cras semper. Tellus in hac habitasse platea. Aliquet bibendum enim facilisis gravida neque convallis. Tempus quam pellentesque nec nam aliquam sem et tortor. Egestas sed sed risus pretium quam vulputate. Bibendum ut tristique et egestas quis ipsum.

Turpis tincidunt id aliquet risus feugiat in ante metus dictum. Nunc lobortis mattis aliquam faucibus purus in. Aliquam sem fringilla ut morbi tincidunt augue interdum velit euismod. Nisi scelerisque eu ultrices vitae auctor eu augue ut. Est ante in nibh mauris cursus mattis molestie a iaculis. Non consectetur a erat nam at lectus. Odio facilisis mauris sit amet massa vitae. Feugiat nibh sed pulvinar proin. Egestas fringilla phasellus faucibus scelerisque eleifend donec pretium. Eget nullam non nisi est sit. Nunc vel risus commodo viverra. Lobortis mattis aliquam faucibus purus in massa. Sapien pellentesque habitant morbi tristique senectus.

Tellus in hac habitasse platea dictumst vestibulum rhoncus. Consectetur a erat nam at lectus urna duis. Nunc eget lorem dolor sed viverra ipsum nunc aliquet. Vitae et leo duis ut diam quam nulla. Pulvinar etiam non quam lacus. Consequat id porta nibh venenatis cras. Venenatis cras sed felis eget velit aliquet sagittis. Libero justo laoreet sit amet cursus sit. Rhoncus est pellentesque elit ullamcorper dignissim cras tincidunt. Nulla facilisi nullam vehicula ipsum. Lectus sit amet est placerat in egestas. Egestas quis ipsum suspendisse ultrices gravida dictum fusce ut placerat. Aenean euismod elementum nisi quis. Duis ultricies lacus sed turpis tincidunt id aliquet. Ipsum nunc aliquet bibendum enim facilisis gravida. Felis donec et odio pellentesque diam volutpat commodo sed egestas. Feugiat nisl pretium fusce id velit ut tortor. Eget nunc lobortis mattis aliquam faucibus purus. Ac tincidunt vitae semper quis lectus nulla at volutpat.

Ipsum nunc aliquet bibendum enim facilisis gravida neque convallis. Non consectetur a erat nam at lectus urna duis convallis. Non sodales neque sodales ut etiam sit amet. Fermentum dui faucibus in ornare quam viverra. Imperdiet nulla malesuada pellentesque elit eget gravida cum sociis. At volutpat diam ut venenatis tellus in metus vulputate eu. Volutpat blandit aliquam etiam erat velit scelerisque in. Ipsum dolor sit amet consectetur adipiscing elit pellentesque. Pharetra convallis posuere morbi leo urna molestie at elementum. Fames ac turpis egestas integer. Suscipit tellus mauris a diam maecenas sed enim ut sem.

Semper quis lectus nulla at volutpat. Nibh cras pulvinar mattis nunc sed. Amet volutpat consequat mauris nunc congue nisi. Tincidunt tortor aliquam nulla facilisi cras fermentum. Pulvinar etiam non quam lacus suspendisse faucibus interdum. Suspendisse ultrices gravida dictum fusce. Scelerisque mauris pellentesque pulvinar pellentesque habitant morbi tristique senectus et. Lacus vestibulum sed arcu non odio euismod lacinia at. Dolor sit amet consectetur adipiscing elit. Enim sit amet venenatis urna cursus eget nunc scelerisque. Tortor consequat id porta nibh venenatis cras sed felis. Amet tellus cras adipiscing enim eu turpis. Nunc mattis enim ut tellus elementum. Natoque penatibus et magnis dis parturient montes nascetur ridiculus. Nam aliquam sem et tortor consequat id porta.

Arcu non odio euismod lacinia at quis risus sed. Blandit cursus risus at ultrices. Facilisis volutpat est velit egestas dui. Varius sit amet mattis vulputate enim nulla. Nibh cras pulvinar mattis nunc sed blandit libero. Faucibus a pellentesque sit amet porttitor eget dolor morbi non. Id volutpat lacus laoreet non curabitur. Morbi blandit cursus risus at ultrices mi tempus. Praesent elementum facilisis leo vel fringilla est ullamcorper eget nulla. Sed odio morbi quis commodo odio.

Vitae tempus quam pellentesque nec nam aliquam. At augue eget arcu dictum varius duis at. Velit ut tortor pretium viverra suspendisse potenti. In dictum non consectetur a erat nam. Nisi lacus sed viverra tellus in hac habitasse. Sit amet nisl purus in. Congue nisi vitae suscipit tellus. Condimentum id venenatis a condimentum vitae sapien. Auctor urna nunc id cursus metus aliquam eleifend. Purus semper eget duis at tellus at urna. Eu consequat ac felis donec et odio pellentesque. Amet venenatis urna cursus eget. Sit amet dictum sit amet justo donec."
        ;8].to_vec();
        let mut value =String::new();
        for imspum in lorem{
            value.push_str(imspum);
        }
        println!("{}", self.send_message(String::from("127.0.0.1:49999"), value.to_string()));

        thread::sleep(Duration::from_secs(5));
        thread::sleep(Duration::from_secs(1));
        self.shutdown();
        println!("Done");
    }
}












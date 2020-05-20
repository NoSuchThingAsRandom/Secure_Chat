extern crate simplelog;
#[macro_use]
extern crate log;
use secure_chat_lib;

use log::LevelFilter;

use simplelog::*;
use std::fs::File;

use std::thread;
use tokio::sync::mpsc::channel;

pub fn main() {
    CombinedLogger::init(
        vec![
            TermLogger::new(LevelFilter::Info, Config::default(), TerminalMode::Stdout),
            WriteLogger::new(LevelFilter::Info, Config::default(), File::create("My_log.log").unwrap()),
        ]
    ).unwrap();
    info!("Initiating Setup!");
    let (new_client_sender, new_client_receiver)=channel(100);
    thread::spawn(|| {
        info!("Starting input loop");
        secure_chat_lib::input_loop(new_client_receiver);
    });
    secure_chat_lib::network::listening_server(new_client_sender);
}

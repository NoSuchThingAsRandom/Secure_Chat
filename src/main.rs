extern crate simplelog;
#[macro_use]
extern crate log;

use secure_chat_lib;

use log::LevelFilter;
use simplelog::*;

use std::fs::{File, read};

use std::{thread, time};


use std::error::Error;

use std::sync::mpsc::{channel, TryRecvError};
use mio::net::TcpStream;
use std::net::SocketAddr;
use text_io::read;
use secure_chat_lib::InputLoop;
use std::time::Duration;


pub fn main() {
    let mut config = ConfigBuilder::new();
    config.set_location_level(LevelFilter::Error);
    config.set_thread_level(LevelFilter::Error);
    config.set_time_level(LevelFilter::Error);
    CombinedLogger::init(
        vec![
            //TermLogger::new(LevelFilter::Trace, config.build(), TerminalMode::Stdout),
            WriteLogger::new(LevelFilter::Error, config.build(), File::create("Logs/master.log").unwrap()),
        ]
    ).unwrap();
    let start=chrono::Utc::now().timestamp_millis();
    thread::sleep(Duration::from_secs(1));
    let end=chrono::Utc::now().timestamp_millis();
    println!("{:?}",(end-start));
    println!("{:?}",chrono::Utc::now().timestamp());
    info!("Initiating Setup!");
    let mut input_loop = InputLoop::new(String::from("127.0.0.1:49999"));
    input_loop.test();
}

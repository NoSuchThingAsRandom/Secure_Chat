#[macro_use]
extern crate log;
extern crate simplelog;

use std::fs::File;
use std::thread;
use std::time::Duration;

use log::LevelFilter;
use simplelog::*;

use secure_chat_lib;
use secure_chat_lib::InputLoop;

fn start_tests() {
    thread::spawn(move || {
        let mut test = InputLoop::new(String::from("127.0.0.1:49998"));
        test.fish();
    });
    thread::sleep(Duration::from_secs(5));
    thread::sleep(Duration::from_secs(10));


    //input_loop.test_multi_server_multi_client();
    //input_loop.test_single_server_multi_client();
}

pub fn main() {
    let mut config = ConfigBuilder::new();
    config.set_location_level(LevelFilter::Error);
    config.set_thread_level(LevelFilter::Error);
    config.set_time_level(LevelFilter::Error);
    CombinedLogger::init(
        vec![
            TermLogger::new(LevelFilter::Trace, config.build(), TerminalMode::Stdout),
            WriteLogger::new(LevelFilter::Error, config.build(), File::create("Logs/master.log").unwrap()),
        ]
    ).unwrap();


    let start = chrono::Utc::now().timestamp_millis();
    thread::sleep(Duration::from_secs(1));
    let end = chrono::Utc::now().timestamp_millis();
    println!("{:?}", (end - start));
    println!("{:?}", chrono::Utc::now().timestamp());
    info!("Initiating Setup!");
    InputLoop::new(String::from("127.0.0.1:49999"));
    start_tests();
    loop {}
}

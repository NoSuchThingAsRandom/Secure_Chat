use secure_chat_lib;
use secure_chat_lib::{network, check_messages, update_clients};
use std::thread;
use secure_chat_lib::network::Client;
use tokio::sync::mpsc::channel;
use log::info;
#[test]
fn recieve_client_connection(){
    let (mut new_client_sender, mut new_client_receiver)=channel(10);
    let messages=vec!("Hello","How are you?");
    thread::spawn(move|| {
        let mut clients :Vec<Client>=Vec::new();
        assert_eq!(2,3);
        secure_chat_lib::network::listening_server(new_client_sender);
        info!("Updating clients");
        update_clients(&mut new_client_receiver,&mut clients);
        info!("Client size {}",clients.len());
        assert_eq!(2,clients.len());
        assert_eq!(2,3)
    });

    //assert_eq!(4,network::test_this())
}#[test]
fn recieve_messages(){
    let (new_client_sender, new_client_receiver)=channel(10);
    let messages=vec!("Hello","How are you?");
    thread::spawn(|| {
        let mut clients :Vec<Client>=Vec::new();
        secure_chat_lib::network::listening_server(new_client_sender);
        check_messages(&mut clients);

    });

    assert_eq!(4,network::test_this())
}
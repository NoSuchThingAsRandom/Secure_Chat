use secure_chat_lib;
use secure_chat_lib::network;

#[test]
fn sample(){
    assert_eq!(4,network::test_this())
}
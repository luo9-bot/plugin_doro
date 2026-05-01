use luo9_sdk::bus::Bus;
use luo9_sdk::payload::*;

use crate::{handle_group_msg, init_data};

#[unsafe(no_mangle)]
pub extern "C" fn plugin_main() {
    init_data();

    let sub = Bus::topic("luo9_message").subscribe().unwrap();
    let topic = Bus::topic("luo9_message");

    println!("[doro] 插件已启动，监听消息中...");

    loop {
        if let Some(json) = topic.pop(sub) {
            if let Some(BusPayload::Message(msg)) = BusPayload::parse(&json) {
                if let MsgType::Group = msg.message_type {
                    handle_group_msg(msg.group_id.unwrap_or(0), msg.user_id, &msg.message);
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

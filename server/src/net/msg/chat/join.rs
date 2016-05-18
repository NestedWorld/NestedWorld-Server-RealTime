use net::msg::{MessagePart, MessageHeader};
use net::msg::error::Result;
use net::msg::states::auth::Authenticated;
use net::msg::utils::fields;
use net::msg::utils::rmp::ValueExt;
use rmp::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct JoinChannel {
    pub header: MessageHeader,
    pub auth: Authenticated,
    pub channel: String,
}

impl MessagePart for JoinChannel {
    fn decode(data: &Value) -> Result<JoinChannel> {
        Ok(JoinChannel {
            header: try!(MessageHeader::decode(data)),
            auth: try!(Authenticated::decode(data)),
            channel: try!(fields::get(data, "channel")),
        })
    }

    fn encode(&self, data: &mut Value) {
        data.set("type", "chat:join-channel");
        self.header.encode(data);
        self.auth.encode(data);
        data.set("channel", &self.channel);
    }
}

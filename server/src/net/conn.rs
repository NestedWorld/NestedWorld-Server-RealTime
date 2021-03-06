use ctx::Context;
use db::models::token::Session;
use mioco;
use mioco::sync::mpsc as chan;
use mioco::tcp::TcpStream;
use super::msg::{Message, MessagePart, MessageFull};
use rmp::decode::value::read_value;
use rmp::encode::value::Error as EncodeError;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use super::handlers;
use super::event;

pub struct Connection {
    pub ctx: Context,
    pub stream: TcpStream,
    pub session: Option<Session>,
    conversations: Arc<Mutex<HashMap<String, chan::Sender<Message>>>>,
}

impl Connection {
    pub fn new(ctx: Context, stream: TcpStream) -> Connection {
        Connection {
            ctx: ctx,
            stream: stream,
            session: None,
            conversations: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn send<M: MessagePart>(&mut self, msg: M) -> Result<(), EncodeError> {
        use rmp::encode::value::write_value;
        write_value(&mut self.stream, &msg.value())
    }

    pub fn send_request<M: MessageFull>(&mut self, mut msg: M) -> Result<chan::Receiver<Message>, EncodeError> {
        let id = msg.header_mut().ensure_id();

        self.send(msg).map(|_| {
            let (tx, rx) = chan::channel();
            let mut conversations = self.conversations.lock().unwrap();
            conversations.insert(id, tx);
            rx
        })
    }

    pub fn get_conversation(&mut self, id: &str) -> Option<chan::Sender<Message>> {
        let mut conversations = self.conversations.lock().unwrap();
        conversations.remove(id)
    }

    pub fn try_clone(&self) -> ::std::io::Result<Connection> {
        Ok(Connection {
          ctx: self.ctx.clone(),
          stream: try!(self.stream.try_clone()),
          session: self.session.clone(),
          conversations: self.conversations.clone(),
        })
    }
}

pub fn run(ctx: Context, conn: TcpStream) {
    let conn = Connection::new(ctx, conn);

    debug!("Got connection!");

    match conn.try_clone() {
        Ok(mut conn) => {
            mioco::spawn(move || read_and_decode(&mut conn));
        },
        Err(err) => {
            debug!("Error when trying to clone TcpStream connection : {}", err);
        },
    }

    match conn.try_clone() {
        Ok(mut conn) => {
            mioco::spawn(move || event::send_random_combat(&mut conn));
        },
        Err(err) => {
            debug!("Error when trying to clone TcpStream connection : {}", err);
        },
    }
}

pub fn read_and_decode(conn: &mut Connection) {
    loop {
        let msg = match read_value(&mut conn.stream) {
            Ok(msg) => msg,
            Err(e) => {
                // Error during reading value, we just handle this silently by closing the
                // connection.
                debug!("Error reading MessagePack value: {}", e);
                break;
            }
        };

        let msg = match Message::decode(&msg) {
            Ok(msg) => msg,
            Err(e) => {
                debug!("Received an invalid message: {}", e);
                continue;
            }
        };

        handlers::handle(conn, msg);
    }
}

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use color_eyre::eyre::Result;
use log::info;

use crate::cmdline::{self, ClientCommand};
use crate::common::{self, get_ws_builder, Address, Message, Request, WSMsg};
use crate::error::{DoodleError, AsEyreErrorResult};

pub struct Client {
    sender: Arc<Mutex<Option<ws::Sender>>>,
    thread: Option<thread::JoinHandle<()>>,
    recv_channel: Receiver<WSMsg>,
    #[allow(dead_code)]
    inner: ClientInner,
}

impl Drop for Client {
    fn drop(&mut self) {
        self.close()
    }
}

trait Mailbox {
    fn send(&mut self, msg: WSMsg);
}

struct ClientInner {
    mailbox: Arc<Mutex<Box<dyn Mailbox + Send>>>,
}

impl ws::Handler for ClientInner {
    fn on_open(&mut self, _shake: ws::Handshake) -> ws::Result<()> {
        let _ = self.mailbox.lock().unwrap().send(WSMsg::Open);
        Ok(())
    }

    fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {
        let decoded_msg: Message = serde_json::from_str(&msg.to_string()).unwrap();
        let _ = self
            .mailbox
            .lock()
            .unwrap()
            .send(WSMsg::Message(decoded_msg));
        Ok(())
    }

    fn on_shutdown(&mut self) {
        let _ = self.mailbox.lock().unwrap().send(WSMsg::Shutdown);
    }

    fn on_close(&mut self, code: ws::CloseCode, reason: &str) {
        let _ = self
            .mailbox
            .lock()
            .unwrap()
            .send(WSMsg::Close(code, reason.to_owned()));
    }

    fn on_error(&mut self, err: ws::Error) {
        let _ = self.mailbox.lock().unwrap().send(WSMsg::Error(err));
    }

    fn on_timeout(&mut self, _event: ws::util::Token) -> ws::Result<()> {
        let _ = self.mailbox.lock().unwrap().send(WSMsg::Timeout);
        Ok(())
    }
}

pub fn send_json_message(message: &Message, sender: &ws::Sender) -> Result<()> {
    let serialized = serde_json::to_string(&message).unwrap_or_else(|e| {
        panic!("to_string failed on \"{}\" with {:?} as input", e, message);
    });
    sender.send(serialized).as_eyre_result()
}

impl Client {
    pub fn recv(&self) -> Result<WSMsg> {
        self.recv_channel.recv().as_eyre_result()
    }

    pub fn send(&self, message: Message) -> Result<()> {
        let sender = self.sender.lock().unwrap();
        match &*sender {
            None => Ok(()),
            Some(sender) => send_json_message(&message, sender),
        }
    }

    pub fn close(&mut self) {
        let sender = self.sender.lock().unwrap();
        match &*sender {
            None => {}
            Some(sender) => {
                let _ = sender.close(ws::CloseCode::Normal);
            }
        }

        match self.thread.take() {
            None => {}
            Some(th) => {
                let _ = th.join();
            }
        }
    }

    pub fn new(address: &Address) -> Result<Self> {
        let (tx, rx) = channel();
        let tx_err = tx.clone();
        let sender_arc = Arc::new(Mutex::new(None));

        struct WrapSender(Sender<WSMsg>);
        impl Mailbox for WrapSender {
            fn send(&mut self, msg: WSMsg) {
                let _ = self.0.send(msg);
            }
        }
        let b: Box<dyn Mailbox + Send> = Box::new(WrapSender(tx));
        let mailbox = Arc::new(Mutex::new(b));

        let mut client = Client {
            sender: sender_arc.clone(),
            thread: None,
            recv_channel: rx,
            inner: ClientInner {
                mailbox: mailbox.clone(),
            },
        };

        let mut ws = get_ws_builder(1).build(move |out: ws::Sender| {
            {
                let mut arc = sender_arc.lock().unwrap();
                *arc = Some(out.clone());
            };

            ClientInner {
                mailbox: mailbox.clone(),
            }
        })?;

        let parsed = url::Url::parse(&format!("ws://{}:{}", address.host, address.port))
            .as_eyre_result()?;
        let th = thread::Builder::new()
            .name("client".to_owned())
            .spawn(move || {
                match ws.connect(parsed) {
                    Ok(_) => {}
                    Err(err) => {
                        let _ = tx_err.send(WSMsg::InitError(err));
                        return;
                    }
                }
                match ws.run() {
                    Ok(_) => {}
                    Err(err) => {
                        let _ = tx_err.send(WSMsg::InitError(err));
                        return;
                    }
                }

                info!("Ending client thread");
            })
            .and_then(|_| Err(std::io::Error::from_raw_os_error(2)))
            .as_eyre_result()?;

        client.thread = Some(th);
        Ok(client)
    }
}

pub fn make_message(command: &cmdline::Client) -> Result<Message> {
    Ok(Message::Request(match &command.command {
        ClientCommand::Play(_) => Request::Play(common::PlayReq),
        ClientCommand::Pause | ClientCommand::Queue(_) | ClientCommand::Status => {
            Err(DoodleError::Generic("Not Implemented".to_owned()))?
        }
        ClientCommand::Shutdown => Request::Shutdown,
    }))
}

pub(crate) fn main(command: cmdline::Client, server_address: Address) -> Result<()> {
    info!("running {:?} with server {}", command, server_address);

    let message = make_message(&command)?;

    let client = Client::new(&server_address)?;
    match client.recv()? {
        WSMsg::Open => {}
        connect_rsp => return Err(DoodleError::NoOpen(connect_rsp))?,
    }

    client.send(message)?;

    let rsp = client.recv()?;
    if let WSMsg::Message(Message::Response(response)) = rsp {
        info!("{:#?}", response); // TODO: replace with debug!(...) when we have real handling
    } else {
        return Err(DoodleError::UnexpectedResponse(rsp))?;
    }

    Ok(())
}

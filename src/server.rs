use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use color_eyre::eyre::Result;
use log::{error, info, warn};

use crate::cmdline;
use crate::common::{
    self, get_ws_builder, Address, ConnId, Message, Request, Response, ServerRequest, WSEvent,
};
use crate::error::AsEyreErrorResult;

pub trait ServerHandler {
    fn on_open(&mut self, _: Address, _: ConnId);
    fn on_remote_call(&mut self, _: Message, _: ConnId, sender: &ws::Sender);
    fn on_event(&mut self, _: ConnId, event: WSEvent, sender: &ws::Sender);
}

pub type HandlerDyn = Arc<Mutex<dyn ServerHandler + Send>>;

struct IncomingComm {
    id: u64,
    handler: HandlerDyn,
    sender: ws::Sender,
    shutdown: bool,
}

impl ws::Handler for IncomingComm {
    fn on_open(&mut self, shake: ws::Handshake) -> ws::Result<()> {
        let mut handler = self.handler.lock().unwrap();
        let address = Address {
            host: shake
                .remote_addr()
                .unwrap_or_else(|e| Some(format!("<error:{}>", e)))
                .unwrap_or_else(|| "<unknown>".to_string()),
            port: 0,
        };
        handler.on_open(address, ConnId(self.id));
        Ok(())
    }

    fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {
        let decoded_msg: Message = serde_json::from_str(&msg.to_string()).unwrap();
        let mut handler = self.handler.lock().unwrap();
        handler.on_remote_call(decoded_msg, ConnId(self.id), &self.sender);
        Ok(())
    }

    fn on_close(&mut self, code: ws::CloseCode, reason: &str) {
        let mut handler = self.handler.lock().unwrap();
        handler.on_event(
            ConnId(self.id),
            WSEvent::Close(code, reason.to_owned()),
            &self.sender,
        );
    }

    fn on_shutdown(&mut self) {
        if !self.shutdown {
            self.shutdown = true;
            let mut handler = self.handler.lock().unwrap();
            handler.on_event(ConnId(self.id), WSEvent::Shutdown, &self.sender);
        }
    }

    fn on_error(&mut self, err: ws::Error) {
        let mut handler = self.handler.lock().unwrap();
        handler.on_event(ConnId(self.id), WSEvent::Error(err), &self.sender);
    }

    fn on_timeout(&mut self, _event: ws::util::Token) -> ws::Result<()> {
        let mut handler = self.handler.lock().unwrap();
        handler.on_event(ConnId(self.id), WSEvent::Timeout, &self.sender);
        Ok(())
    }
}

#[derive(Default)]
pub struct Connections {
    next_conn_id: u64,
}

impl Connections {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn next_conn_id(&mut self) -> u64 {
        let id = self.next_conn_id;
        self.next_conn_id += 1;
        id
    }
}

#[derive(Debug)]
struct CallCompletion {
    conn_id: ConnId,
    sender: ws::Sender,
}

#[derive(Debug)]
struct ResponseWrapper {
    response: Response,
    shutdown: bool,
}

impl From<Response> for ResponseWrapper {
    fn from(response: Response) -> Self {
        Self::new(response)
    }
}

impl ResponseWrapper {
    fn new(response: Response) -> Self {
        Self {
            response,
            shutdown: false,
        }
    }
    fn with_shutdown(mut self) -> Self {
        self.shutdown = true;
        self
    }
}

impl CallCompletion {
    fn complete(&self, resp: ResponseWrapper) {
        match common::send_json_message(&Message::Response(resp.response), &self.sender) {
            Err(err) => error!("{:?} - error {:?} sending response", self.conn_id, err),
            _ => info!("{:?} - sent response to client", self.conn_id),
        }

        if resp.shutdown {
            match self.sender.close(ws::CloseCode::Normal) {
                Err(err) => error!("{:?} - error {:?} closing", self.conn_id, err),
                _ => info!("{:?} - closed due to shutdown", self.conn_id),
            }
        }
    }
}

pub fn server_spawn(
    address: &Address,
    handler: HandlerDyn,
) -> Result<(u16, thread::JoinHandle<Result<()>>)> {
    let connections = Arc::new(Mutex::new(Connections::new()));

    let ws = get_ws_builder(2000).build(move |sender| {
        let mut connections = connections.lock().unwrap();
        let id = connections.next_conn_id();

        IncomingComm {
            id,
            sender,
            handler: handler.clone(),
            shutdown: false,
        }
    })?;

    let ws = ws
        .bind(format!("{}:{}", address.host, address.port))
        .as_eyre_result()?;
    let local_addr = ws.local_addr().as_eyre_result()?;
    let port = local_addr.port();

    info!("Listening on {}", local_addr);

    let th = thread::Builder::new()
        .name("server".to_owned())
        .spawn(move || {
            let x = ws.run().map(|_| {});

            info!("Ending listening thread");

            x.as_eyre_result()
        })
        .as_eyre_result()?;

    Ok((port, th))
}

pub struct PlayerThread {
    #[allow(dead_code)]
    currently_playing: Option<String>,
    receiver: mpsc::Receiver<ServerRequest>,
    #[allow(dead_code)]
    sender: mpsc::Sender<ServerRequest>,
    shutdown: bool,
}

impl PlayerThread {
    pub fn new(
        receiver: mpsc::Receiver<ServerRequest>,
        sender: mpsc::Sender<ServerRequest>,
    ) -> Self {
        Self {
            currently_playing: None,
            receiver,
            sender,
            shutdown: false,
        }
    }

    fn on_remote_call(&mut self, request: Request, call_completion: CallCompletion) {
        match request {
            Request::Play(play_info) => {
                self.play(play_info, call_completion);
            }
            Request::Shutdown => {
                info!("Shutting down...");
                self.shutdown = true;
                call_completion.complete(ResponseWrapper::new(Response::Ok).with_shutdown());
            }
        }
    }

    fn play(&mut self, play_info: common::PlayReq, call_completion: CallCompletion) {
        info!("TODO: play something");
        drop(play_info);
        call_completion.complete(Response::Ok.into());
    }

    pub fn run(&mut self) {
        let mut ws_sender = None;
        while let Ok(ServerRequest(request, conn_id, sender)) = self.receiver.recv() {
            info!("{:?} - {:?}", conn_id, request);
            if ws_sender.is_none() {
                ws_sender = Some(sender.clone());
            }
            self.on_remote_call(request, CallCompletion { conn_id, sender });
            if self.shutdown {
                std::thread::sleep(std::time::Duration::from_millis(1)); // Prevent Abnormal close on the client's side
                let _ = ws_sender
                    .take()
                    .map(|sender| sender.shutdown())
                    .expect("shutdown without any connection");
                break;
            }
        }

        info!("Main player thread ended");
    }
}

pub struct Server {
    #[allow(dead_code)]
    path: std::path::PathBuf,
    sender: mpsc::Sender<ServerRequest>,
    _thread: thread::JoinHandle<()>,
}

impl Server {
    pub fn new(path: std::path::PathBuf) -> Self {
        let (tx, rx) = mpsc::channel();

        Self {
            path,
            sender: tx.clone(),
            _thread: thread::Builder::new()
                .name("player".to_owned())
                .spawn(move || {
                    let mut inner = PlayerThread::new(rx, tx.clone());
                    inner.run()
                })
                .as_eyre_result()
                .expect("failed to start player thread"),
        }
    }
}

impl ServerHandler for Server {
    fn on_open(&mut self, address: Address, conn_id: ConnId) {
        info!("{:?} - open from {:?}", conn_id, address);
    }

    fn on_remote_call(&mut self, msg: Message, conn_id: ConnId, sender: &ws::Sender) {
        match msg {
            Message::Request(req) => {
                self.sender
                    .send(ServerRequest(req, conn_id, sender.clone()))
                    .as_eyre_result()
                    .unwrap();
            }
            Message::Response(..) => {
                warn!("{:?} - Ignoring unexpected {:?}", conn_id, msg);
            }
        }
    }

    fn on_event(&mut self, conn_id: ConnId, event: WSEvent, _sender: &ws::Sender) {
        match &event {
            WSEvent::Shutdown | WSEvent::Close(..) => {
                info!("{:?} - {:?}", conn_id, event);
            }
            WSEvent::Timeout | WSEvent::Error(..) => {
                error!("{:?} - {:?}", conn_id, event);
            }
        }

        // info!("shutting down after last client event");
        // let _ = sender.shutdown();
    }
}

pub(crate) fn main(command: cmdline::Server, address: Address) -> Result<()> {
    info!("running {:?} as server on {}", command, address);

    let server = Arc::new(Mutex::new(Server::new(command.path)));

    let (_, th) = server_spawn(&address, server)?;

    match th.join() {
        Ok(result) => result,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

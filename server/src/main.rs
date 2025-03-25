use std::{
    collections::HashMap,
    error::Error,
    io,
    net::{SocketAddr, ToSocketAddrs},
    time::Duration,
};

use log::{error, info, warn};
use message_io::{
    network::{Endpoint, NetEvent, ResourceId, SendStatus, Transport},
    node::{self, NodeHandler, NodeListener},
};

use common::{
    message::{BoardMessage, DndMessage, Log, LogMessage, UnRegisterUser},
    Ability, Character, DndPlayerPiece, Item, User,
};
use postgrest::Postgrest;

mod db_types;
mod tasks;
use db_types::*;
use thiserror::Error;

const AUTOSAVE_TIME_IN_SECS: u64 = 30;

pub trait ToError<T> {
    fn to_error(self) -> anyhow::Result<T>;
}

impl ToError<()> for SendStatus {
    fn to_error(self) -> anyhow::Result<()> {
        match self {
            SendStatus::Sent => Ok(()),
            _ => Err(ServerError::ResponseError(self).into()),
        }
    }
}

impl ToError<reqwest::Response> for reqwest::Response {
    fn to_error(self) -> anyhow::Result<Self> {
        let res = self.error_for_status()?;
        Ok(res)
    }
}

pub trait ResponseTextWithError {
    #[allow(async_fn_in_trait)]
    async fn text_with_error(self) -> anyhow::Result<String>;
}

impl ResponseTextWithError for reqwest::Response {
    async fn text_with_error(self) -> anyhow::Result<String> {
        self.error_for_status()?.text().await.map_err(|x| x.into())
    }
}

struct ClientInfo {
    user_data: User,
    endpoint: Endpoint,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    env_logger::init();
    let server = DndServer::new("0.0.0.0", 80)?;
    server.run();

    Ok(())
}

type PlayerLookup = HashMap<uuid::Uuid, DndPlayerPiece>;

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Default)]
struct BoardData {
    #[serde(skip)]
    dirty: bool,
    players: PlayerLookup,
}

impl BoardData {
    pub fn insert_player(&mut self, uuid: uuid::Uuid, player: DndPlayerPiece) {
        self.dirty = true;
        self.players.insert(uuid, player);
    }

    pub fn get_player_mut(
        &mut self,
        uuid: &uuid::Uuid,
    ) -> Result<&mut DndPlayerPiece, ServerError> {
        self.dirty = true;
        self.players
            .get_mut(uuid)
            .ok_or(ServerError::PlayerNotFound(*uuid))
    }

    pub fn remove_player(&mut self, uuid: &uuid::Uuid) {
        self.dirty = true;
        self.players.remove(uuid);
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_clean(&mut self) {
        self.dirty = true;
    }
}

enum Signal {
    Autosave,
}

#[derive(Error, Debug)]
enum ServerError {
    #[error("No board saves exist, starting fresh!")]
    NoBoardSaves,
    #[error("Player {0} could not be found on the server!")]
    PlayerNotFound(uuid::Uuid),
    #[error("Failed to send server response: {0:?}")]
    ResponseError(SendStatus),
    #[error("User already exists: {0}")]
    UserAlreadyExists(String),
    #[error("User cannot be found: {0}")]
    UserNotFound(String),
}

pub struct DndServer {
    handler: NodeHandler<Signal>,
    board_data: BoardData,
    self_endpoint: Endpoint,
    node_listener: Option<NodeListener<Signal>>,
    users: HashMap<String, ClientInfo>,
    db: Postgrest,
}

impl DndServer {
    pub fn new(addr: &str, port: u16) -> io::Result<Self> {
        let (handler, node_listener) = node::split::<Signal>();
        let addr = (addr, port).to_socket_addrs().unwrap().next().unwrap();

        let (ws_id, ws_addr) = handler.network().listen(Transport::Ws, addr)?;

        let url = dotenv::var("NEXT_PUBLIC_SUPABASE_URL").unwrap();
        let db = Postgrest::new(url).insert_header(
            "apikey",
            dotenv::var("NEXT_PUBLIC_SUPABASE_ANON_KEY").unwrap(),
        );

        info!("Connected to DB");

        info!("Server running at {}", addr);

        // Fake endpoint that doesn't really matter
        // Magic number matches bit 7 & 2 set because we need
        // non-connection oriented (2) and local (7)
        let self_endpoint = Endpoint::from_listener(130.into(), ws_addr);

        handler
            .signals()
            .send_with_timer(Signal::Autosave, Duration::from_secs(AUTOSAVE_TIME_IN_SECS));

        let mut server = Self {
            db,
            handler,
            self_endpoint,
            node_listener: Some(node_listener),
            users: HashMap::new(),
            board_data: BoardData::default(),
        };

        server.process_task(server.self_endpoint, tasks::board::GetLatestBoardData);

        Ok(server)
    }

    pub fn run(mut self) {
        let node_listener = self.node_listener.take().unwrap();
        node_listener.for_each(move |event| match event {
            node::NodeEvent::Network(net_event) => match net_event {
                NetEvent::Connected(_, _) => unreachable!(),
                NetEvent::Accepted(_, _) => (),
                NetEvent::Message(endpoint, input_data) => {
                    let message: DndMessage = bincode::deserialize(input_data).unwrap();
                    match message {
                        DndMessage::RegisterUser(msg) => self.process_task(endpoint, msg),
                        DndMessage::UnregisterUser(msg) => self.process_task(endpoint, msg),
                        DndMessage::UserNotificationRemoved(_) => todo!(),
                        DndMessage::RetrieveCharacterData(msg) => self.process_task(endpoint, msg),
                        DndMessage::UpdateItemCount(msg) => self.process_task(endpoint, msg),
                        DndMessage::UpdateAbilityCount(msg) => self.process_task(endpoint, msg),
                        DndMessage::UpdateSkills(msg) => self.process_task(endpoint, msg),
                        DndMessage::UpdateHealth(msg) => self.process_task(endpoint, msg),
                        DndMessage::UpdatePowerSlotCount(msg) => self.process_task(endpoint, msg),
                        DndMessage::BoardMessage(msg) => self.process_task(endpoint, msg),
                        DndMessage::Log(msg) => self.process_task(endpoint, msg),
                        _ => {
                            warn!("Unhandled message {message:?}");
                        }
                    }
                }
                NetEvent::Disconnected(endpoint) => {
                    let user = self
                        .users
                        .iter()
                        .find(|(_, info)| info.endpoint == endpoint);

                    if let Some((name, _)) = user {
                        let name = name.clone();

                        self.process_task(endpoint, UnRegisterUser { name });
                    }
                }
            },
            node::NodeEvent::Signal(signal) => match signal {
                Signal::Autosave => {
                    info!("Autosaving...");

                    self.process_task(self.self_endpoint, tasks::board::SaveBoardData);

                    self.handler.signals().send_with_timer(
                        Signal::Autosave,
                        Duration::from_secs(AUTOSAVE_TIME_IN_SECS),
                    );
                }
            },
        });
    }

    fn process_task<T: tasks::ServerTask>(&mut self, endpoint: Endpoint, task: T) {
        let res = futures::executor::block_on(task.process(endpoint, self));

        if let Err(e) = res {
            error!("Error handling server task: {e}");
        }
    }
}

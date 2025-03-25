use std::{
    collections::{hash_map::Values, HashMap},
    error::Error,
    io,
    net::{SocketAddr, ToSocketAddrs},
    sync::{atomic::AtomicBool, Arc, Mutex, RwLock, RwLockReadGuard},
    time::Duration,
};

use emath::Pos2;
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
use tasks::ServerTask;
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

#[derive(Clone)]
pub struct ClientInfo {
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
    dirty: Arc<AtomicBool>,
    players: Arc<RwLock<PlayerLookup>>,
}

impl BoardData {
    fn mark_dirty(&self) {
        self.dirty.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn mark_clean(&self) {
        self.dirty.store(false, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn insert_player(&self, uuid: uuid::Uuid, player: DndPlayerPiece) {
        let mut players = self.players.write().unwrap();
        players.insert(uuid, player);
        self.mark_dirty();
    }

    pub fn update_player(
        &self,
        uuid: &uuid::Uuid,
        new_player: DndPlayerPiece,
    ) -> Result<(), ServerError> {
        let mut players = self.players.write().unwrap();
        let player = players
            .get_mut(uuid)
            .ok_or(ServerError::PlayerNotFound(*uuid))?;

        *player = new_player;
        self.mark_dirty();

        Ok(())
    }

    pub fn update_player_location(
        &self,
        uuid: &uuid::Uuid,
        new_location: Pos2,
    ) -> Result<(), ServerError> {
        let mut players = self.players.write().unwrap();
        let player = players
            .get_mut(uuid)
            .ok_or(ServerError::PlayerNotFound(*uuid))?;

        player.position = new_location;
        self.mark_dirty();

        Ok(())
    }

    pub fn remove_player(&self, uuid: &uuid::Uuid) {
        self.mark_dirty();

        let mut players = self.players.write().unwrap();
        players.remove(uuid);
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn get_players_owned(&self) -> HashMap<uuid::Uuid, DndPlayerPiece> {
        self.players.read().unwrap().clone()
    }

    pub fn overwrite_board_data(&self, new_data: BoardData) {
        let mut players = self.players.write().unwrap();
        *players = new_data.players.read().unwrap().clone();
    }
}

#[derive(Default, Clone)]
pub struct UserData {
    user_list: Arc<RwLock<HashMap<String, ClientInfo>>>,
}

impl UserData {
    pub fn foreach<F>(&self, f: F) -> anyhow::Result<()>
    where
        F: Fn((&String, &ClientInfo)) -> anyhow::Result<()>,
    {
        let users = self.user_list.read().unwrap();

        for u in users.iter() {
            f(u)?;
        }

        Ok(())
    }

    pub fn has_user(&self, name: &String) -> bool {
        let users = self.user_list.read().unwrap();
        users.contains_key(name)
    }

    pub fn insert_user(&self, name: String, user_data: ClientInfo) {
        let mut users = self.user_list.write().unwrap();
        users.insert(name, user_data);
    }

    pub fn remove_user(&self, name: &String) -> Option<ClientInfo> {
        let mut users = self.user_list.write().unwrap();
        users.remove(name)
    }

    pub fn find_name_for_endpoint(&self, endpoint: Endpoint) -> Option<String> {
        let users = self.user_list.read().unwrap();
        let found = users.iter().find(|(_, info)| info.endpoint == endpoint);
        found.map(|(name, _)| name).cloned()
    }

    pub fn users_names_owned(&self) -> Vec<String> {
        let users = self.user_list.read().unwrap();
        users.keys().cloned().collect()
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
    users: UserData,
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

        let server = Self {
            db,
            handler,
            self_endpoint,
            node_listener: Some(node_listener),
            users: UserData::default(),
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
                    let user = self.users.find_name_for_endpoint(endpoint);

                    if let Some(name) = user {
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

    fn process_task<T: tasks::ServerTask>(&self, endpoint: Endpoint, task: T) {
        let res = futures::executor::block_on(task.process(endpoint, self));

        if let Err(e) = res {
            error!("Error handling server task: {e}");
        }
    }
}

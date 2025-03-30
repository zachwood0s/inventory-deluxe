use std::{
    collections::{hash_map::Values, HashMap},
    error::Error,
    io,
    net::{SocketAddr, ToSocketAddrs},
    sync::{atomic::AtomicBool, Arc, Mutex, RwLock, RwLockReadGuard},
    time::Duration,
};

use anyhow::anyhow;
use emath::Pos2;
use log::{error, info, warn};
use message_io::{
    network::{Endpoint, NetEvent, ResourceId, SendStatus, Transport},
    node::{self, NodeHandler, NodeListener},
};

use common::{
    message::{BoardMessage, DndMessage, Log, LogMessage, SaveBoard, UnRegisterUser},
    Ability, Character, DndPlayerPiece, Item, User,
};
use postgrest::Postgrest;

mod db_types;
mod tasks;
use db_types::*;
use tasks::{board::BoardData, ServerTask};
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
    server.run().await;

    Ok(())
}

type PlayerLookup = HashMap<uuid::Uuid, DndPlayerPiece>;

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

        let (_, ws_addr) = handler.network().listen(Transport::Ws, addr)?;

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

    pub async fn run(mut self) {
        let node_listener = self.node_listener.take().unwrap();
        let server = Arc::new(self);

        let autosave = autosave_task(Arc::clone(&server));

        let listener = async {
            node_listener.for_each(move |event| {
                if let node::NodeEvent::Network(net_event) = event {
                    match net_event {
                        NetEvent::Connected(_, _) => unreachable!(),
                        NetEvent::Accepted(_, _) => (),
                        NetEvent::Message(endpoint, input_data) => {
                            let message: DndMessage = bincode::deserialize(input_data).unwrap();
                            server.process_task(endpoint, message);
                        }
                        NetEvent::Disconnected(endpoint) => {
                            let user = server.users.find_name_for_endpoint(endpoint);

                            if let Some(name) = user {
                                let name = name.clone();
                                server.process_task(endpoint, UnRegisterUser { name });
                            }
                        }
                    }
                }
            });
        };

        tokio::join!(autosave, listener);
    }

    fn process_task<T: tasks::ServerTask>(&self, endpoint: Endpoint, task: T) {
        futures::executor::block_on(self.process_task_async(endpoint, task));
    }

    async fn process_task_async<T: tasks::ServerTask>(&self, endpoint: Endpoint, task: T) {
        let res = task.process(endpoint, self).await;

        if let Err(e) = res {
            error!("Error handling server task: {e}");
        }
    }
}

impl ServerTask for DndMessage {
    async fn process(self, endpoint: Endpoint, server: &DndServer) -> anyhow::Result<()> {
        match self {
            DndMessage::RegisterUser(msg) => msg.process(endpoint, server).await,
            DndMessage::UnregisterUser(msg) => msg.process(endpoint, server).await,
            DndMessage::UserNotificationRemoved(_) => todo!(),
            DndMessage::RetrieveCharacterData(msg) => msg.process(endpoint, server).await,
            DndMessage::UpdateItemCount(msg) => msg.process(endpoint, server).await,
            DndMessage::UpdateAbilityCount(msg) => msg.process(endpoint, server).await,
            DndMessage::UpdateSkills(msg) => msg.process(endpoint, server).await,
            DndMessage::UpdateHealth(msg) => msg.process(endpoint, server).await,
            DndMessage::UpdatePowerSlotCount(msg) => msg.process(endpoint, server).await,
            DndMessage::BoardMessage(msg) => msg.process(endpoint, server).await,
            DndMessage::SaveBoard(msg) => msg.process(endpoint, server).await,
            DndMessage::LoadBoard(msg) => msg.process(endpoint, server).await,
            DndMessage::Log(msg) => msg.process(endpoint, server).await,
            _ => Err(anyhow!("Unhandled message {self:?}")),
        }
    }
}

async fn autosave_task(server: Arc<DndServer>) {
    let res = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(AUTOSAVE_TIME_IN_SECS));

        loop {
            interval.tick().await;

            info!("Autosaving...");
            server
                .process_task_async(server.self_endpoint, SaveBoard { tag: None })
                .await;

            info!("Autosave complete...");
        }
    })
    .await;

    if let Err(res) = res {
        error!("Failed to spawn autosave thread: {res}");
    }
}

use std::{
    collections::HashMap,
    io,
    net::{SocketAddr, ToSocketAddrs},
    sync::{Arc, RwLock},
    time::Duration,
};

use anyhow::anyhow;
use log::{error, info};
use message_io::{
    network::{Endpoint, NetEvent, SendStatus, Transport},
    node::{self, NodeHandler, NodeListener},
};

use common::{
    message::{DndMessage, UnRegisterUser},
    User,
};
use postgrest::Postgrest;

mod db_types;
mod tasks;
use db_types::*;
use tasks::{board::ServerBoardData, ServerTask};
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

pub struct ListenerCtx {
    handler: NodeHandler<()>,
}

#[derive(Clone)]
pub struct ClientInfo {
    user_data: User,
    endpoint: DndEndpoint,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    env_logger::init();
    let server = DndServer::new("0.0.0.0", 80)?;
    server.run().await;

    Ok(())
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

    pub fn find_name_for_endpoint(&self, endpoint: DndEndpoint) -> Option<String> {
        let users = self.user_list.read().unwrap();
        let found = users.iter().find(|(_, info)| info.endpoint == endpoint);
        found.map(|(name, _)| name).cloned()
    }

    pub fn users_names_owned(&self) -> Vec<String> {
        let users = self.user_list.read().unwrap();
        users.keys().cloned().collect()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DndEndpoint {
    Client(Endpoint),
    Server,
}

impl From<Endpoint> for DndEndpoint {
    fn from(endpoint: Endpoint) -> Self {
        Self::Client(endpoint)
    }
}

impl DndEndpoint {
    pub fn client(&self) -> anyhow::Result<Endpoint> {
        match self {
            DndEndpoint::Client(endpoint) => Ok(*endpoint),
            DndEndpoint::Server => Err(anyhow!("Endpoint is not a client!")),
        }
    }
}

#[derive(Error, Debug)]
enum ServerError {
    #[error("No board saves exist, starting fresh!")]
    NoBoardSaves,
    #[error("Failed to send server response: {0:?}")]
    ResponseError(SendStatus),
    #[error("User already exists: {0}")]
    UserAlreadyExists(String),
    #[error("User cannot be found: {0}")]
    UserNotFound(String),
}

pub struct DndServer {
    addr: SocketAddr,
    board_data: ServerBoardData,
    users: UserData,
    db: Postgrest,
}

impl DndServer {
    pub fn new(addr: &str, port: u16) -> io::Result<Self> {
        let addr = (addr, port).to_socket_addrs().unwrap().next().unwrap();

        let url = dotenv::var("NEXT_PUBLIC_SUPABASE_URL").unwrap();
        let db = Postgrest::new(url).insert_header(
            "apikey",
            dotenv::var("NEXT_PUBLIC_SUPABASE_ANON_KEY").unwrap(),
        );

        info!("Connected to DB");

        let server = Self {
            addr,
            db,
            users: UserData::default(),
            board_data: ServerBoardData::default(),
        };

        //server.process_task(server.self_endpoint, tasks::board::GetLatestBoardData);

        Ok(server)
    }

    pub async fn run(self) {
        let server = Arc::new(self);

        let autosave = autosave_task(Arc::clone(&server));
        let listener = listener_task(Arc::clone(&server));

        tokio::join!(autosave, listener);
    }

    fn process_task<T: tasks::ServerTask>(&self, ctx: &ListenerCtx, endpoint: Endpoint, task: T) {
        futures::executor::block_on(self.process_task_async(ctx, endpoint, task));
    }

    async fn process_task_async<T: tasks::ServerTask>(
        &self,
        ctx: &ListenerCtx,
        endpoint: Endpoint,
        task: T,
    ) {
        let res = task.process(endpoint.into(), self, ctx).await;

        if let Err(e) = res {
            error!("Error handling server task: {e}");
        }
    }
}

impl ServerTask for DndMessage {
    async fn process(
        self,
        endpoint: DndEndpoint,
        server: &DndServer,
        ctx: &ListenerCtx,
    ) -> anyhow::Result<()> {
        match self {
            DndMessage::RegisterUser(msg) => msg.process(endpoint, server, ctx).await,
            DndMessage::UnregisterUser(msg) => msg.process(endpoint, server, ctx).await,
            DndMessage::RetrieveCharacterData(msg) => msg.process(endpoint, server, ctx).await,
            DndMessage::UpdateCharacterStats(msg) => msg.process(endpoint, server, ctx).await,
            DndMessage::UpdateItemCount(msg) => msg.process(endpoint, server, ctx).await,
            DndMessage::UpdateAbilityCount(msg) => msg.process(endpoint, server, ctx).await,
            DndMessage::UpdateSkills(msg) => msg.process(endpoint, server, ctx).await,
            DndMessage::UpdatePowerSlotCount(msg) => msg.process(endpoint, server, ctx).await,
            DndMessage::BoardMessage(msg) => msg.process(endpoint, server, ctx).await,
            DndMessage::SaveBoard(msg) => msg.process(endpoint, server, ctx).await,
            DndMessage::LoadBoard(msg) => msg.process(endpoint, server, ctx).await,
            DndMessage::Log(msg) => msg.process(endpoint, server, ctx).await,
            _ => Err(anyhow!("Unhandled message {self:?}")),
        }
    }
}

async fn listener_task(server: Arc<DndServer>) {
    loop {
        let server = Arc::clone(&server);

        info!("Listener starting at {}", server.addr);

        let (handler, node_listener) = node::split::<()>();

        handler
            .network()
            .listen(Transport::Ws, server.addr)
            .expect("Failed to open listener");

        let ctx = ListenerCtx { handler };

        node_listener.for_each(move |event| {
            if let node::NodeEvent::Network(net_event) = event {
                match net_event {
                    NetEvent::Connected(_, _) => unreachable!(),
                    NetEvent::Accepted(_, _) => (),
                    NetEvent::Message(endpoint, input_data) => {
                        let message: DndMessage = bincode::deserialize(input_data).unwrap();
                        server.process_task(&ctx, endpoint, message);
                    }
                    NetEvent::Disconnected(endpoint) => {
                        let user = server.users.find_name_for_endpoint(endpoint.into());

                        if let Some(name) = user {
                            let name = name.clone();
                            server.process_task(&ctx, endpoint, UnRegisterUser { name });
                        }
                    }
                }
            }
        });

        error!("!! Listener fucking killed itself... Trying to revive !!");
    }
}

async fn autosave_task(server: Arc<DndServer>) {
    let res = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(AUTOSAVE_TIME_IN_SECS));

        loop {
            interval.tick().await;

            info!("Autosaving...");
            //server
            //    .process_task_async(server.self_endpoint, SaveBoard { tag: None })
            //    .await;

            info!("Autosave complete...");
        }
    })
    .await;

    if let Err(res) = res {
        error!("Failed to spawn autosave thread: {res}");
    }
}

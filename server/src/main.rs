use std::{
    collections::HashMap,
    error::Error,
    io,
    net::{SocketAddr, ToSocketAddrs},
    time::Duration,
};

use log::{error, info, warn};
use message_io::{
    network::{Endpoint, NetEvent, SendStatus, Transport},
    node::{self, NodeHandler, NodeListener},
};

use common::{
    message::{BoardMessage, DndMessage, LogMessage},
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

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Default)]
struct BoardData {
    dirty: bool,
    players: HashMap<uuid::Uuid, DndPlayerPiece>,
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
}

pub struct DndServer {
    handler: NodeHandler<Signal>,
    board_data: BoardData,
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

        handler
            .signals()
            .send_with_timer(Signal::Autosave, Duration::from_secs(AUTOSAVE_TIME_IN_SECS));

        let mut server = Self {
            db,
            handler,
            node_listener: Some(node_listener),
            users: HashMap::new(),
            board_data: BoardData::default(),
        };

        server.process_task(
            Endpoint::from_listener(ws_id, ws_addr),
            tasks::board::GetLatestBoardData,
        );

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
                        DndMessage::RegisterUser(name) => {
                            self.register(&name, endpoint);
                            self.process_task(
                                endpoint,
                                tasks::log::BroadcastLogMsg::new(
                                    User::server(),
                                    LogMessage::Joined(name),
                                ),
                            );
                        }
                        DndMessage::UnregisterUser(name) => {
                            self.unregister(&name);
                        }
                        DndMessage::UserNotificationRemoved(_) => todo!(),
                        DndMessage::Log(user, msg) => {
                            self.process_task(
                                endpoint,
                                tasks::log::BroadcastLogMsg::new(user, msg),
                            );
                        }
                        DndMessage::RetrieveCharacterData(user) => {
                            self.process_task(endpoint, tasks::db::GetItemList(&user));
                            self.process_task(endpoint, tasks::db::GetAbilityList(&user));
                            self.process_task(endpoint, tasks::db::GetCharacterStats(&user));
                            self.process_task(endpoint, tasks::board::SendInitialBoardData);
                        }
                        DndMessage::UpdateItemCount(user, item_id, new_count) => self.process_task(
                            endpoint,
                            tasks::db::UpdateItemCount::new(&user, item_id, new_count),
                        ),
                        DndMessage::UpdateAbilityCount(user, ability_name, count) => self
                            .process_task(
                                endpoint,
                                tasks::db::UpdateAbilityCount::new(&user, &ability_name, count),
                            ),
                        DndMessage::UpdateSkills(user, skill_list) => {
                            self.update_skills(user, skill_list)
                        }
                        DndMessage::UpdateHealth(user, curr_health, max_health) => {
                            self.update_health(user, curr_health, max_health)
                        }
                        DndMessage::UpdatePowerSlotCount(user, count) => {
                            self.update_powerslot_count(user, count.into());
                        }
                        DndMessage::BoardMessage(msg) => self.process_task(endpoint, msg),
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

                        self.unregister(&name);
                        self.process_task(
                            endpoint,
                            tasks::log::BroadcastLogMsg::new(
                                User::server(),
                                LogMessage::Disconnected(name),
                            ),
                        );
                    }
                }
            },
            node::NodeEvent::Signal(signal) => match signal {
                Signal::Autosave => {
                    info!("Autosaving...");

                    match self.save_board() {
                        Ok(_) => info!("Autosaving complete!"),
                        Err(e) => info!("Failed to save the board: {e}"),
                    }

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

    fn register(&mut self, name: &str, endpoint: Endpoint) {
        if !self.users.contains_key(name) {
            let list = self.users.keys().cloned().collect();

            let message = DndMessage::UserList(list);
            let output_data = bincode::serialize(&message).unwrap();
            self.handler.network().send(endpoint, &output_data);

            let character_list = self.get_character_list().unwrap();
            let message = DndMessage::CharacterList(character_list);
            let output_data = bincode::serialize(&message).unwrap();
            self.handler.network().send(endpoint, &output_data);

            // Notify other users about this new user
            let message = DndMessage::UserNotificationAdded(name.to_string());
            let output_data = bincode::serialize(&message).unwrap();
            for (_name, user) in self.users.iter() {
                self.handler.network().send(user.endpoint, &output_data);
            }

            self.users.insert(
                name.to_string(),
                ClientInfo {
                    user_data: User {
                        name: name.to_string(),
                    },
                    endpoint,
                },
            );

            info!("Added user '{}'", name);
        } else {
            info!(
                "User with name '{}' already exists, whart are you doing??",
                name
            );
        }
    }

    fn unregister(&mut self, name: &str) {
        if let Some(info) = self.users.remove(name) {
            let message = DndMessage::UserNotificationRemoved(name.to_string());
            let output_data = bincode::serialize(&message).unwrap();
            for (_name, user) in self.users.iter() {
                self.handler.network().send(user.endpoint, &output_data);
            }

            info!("Removed participant '{}'", name);
        } else {
            error!("Cannot unregister a user '{}' who doesn't exist??", name);
        }
    }

    fn get_character_list(&self) -> Result<Vec<String>, Box<dyn Error>> {
        info!("Retrieving character list");
        let res = futures::executor::block_on(async {
            let resp = self
                .db
                .from("character")
                .select("name")
                .execute()
                .await
                .unwrap();
            resp.text().await
        })?;

        info!("{}", res);

        #[derive(serde::Deserialize)]
        struct Name {
            name: String,
        }

        let names: Vec<Name> = serde_json::from_str(&res)?;
        Ok(names.into_iter().map(|x| x.name).collect())
    }

    fn update_powerslot_count(&self, user: User, new_count: i64) {
        futures::executor::block_on(async {
            self.db
                .from("characters")
                .eq("player", &user.name)
                .update(format!("{{ \"power_slots\": {} }}", new_count))
                .execute()
                .await
                .unwrap();
        });

        info!("{}'s ability uses updated to {}", user.name, new_count);
    }

    fn update_skills(&self, user: User, skill_list: Vec<String>) {
        let Ok(skill_vec) = serde_json::to_string(&skill_list) else {
            error!(">:(");
            return;
        };

        let res = futures::executor::block_on(async {
            let resp = self
                .db
                .from("character")
                .eq("name", &user.name)
                .update(format!("{{ \"skills\": {} }}", skill_vec))
                .execute()
                .await
                .unwrap();
            resp.text().await
        });

        info!("{:?}", res);

        info!("{}'s skills updated to {}", &user.name, skill_vec);
    }

    fn update_health(&self, user: User, curr_health: i16, max_health: i16) {
        let res = futures::executor::block_on(async {
            let resp = self
                .db
                .from("character")
                .eq("name", &user.name)
                .update(format!(
                    "{{ \"curr_hp\": {curr_health}, \"max_hp\": {max_health}}}"
                ))
                .execute()
                .await
                .unwrap();
            resp.text().await
        });

        info!("{:?}", res);

        info!(
            "{}'s health updated to {curr_health}/{max_health}",
            &user.name
        );
    }

    fn save_board(&self) -> Result<(), Box<dyn Error>> {
        let json_board_data = serde_json::to_string(&self.board_data)?;

        futures::executor::block_on(async {
            self.db
                .from("board_data")
                .insert(format!(
                    "{{\"data\": {json_board_data}, \"tag\": \"autosave\" }}"
                ))
                .execute()
                .await
        })?;

        Ok(())
    }
}

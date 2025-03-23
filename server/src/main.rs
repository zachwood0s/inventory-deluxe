use std::{
    collections::HashMap,
    error::Error,
    io,
    net::{SocketAddr, ToSocketAddrs},
    time::Duration,
};

use log::{error, info, warn};
use message_io::{
    network::{Endpoint, NetEvent, Transport},
    node::{self, NodeHandler, NodeListener},
};

use common::{
    message::{BoardMessage, DndMessage, LogMessage},
    Ability, Character, DndPlayerPiece, Item, User,
};
use postgrest::Postgrest;

mod db_types;
use db_types::*;
use thiserror::Error;

const AUTOSAVE_TIME_IN_SECS: u64 = 30;

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
    players: HashMap<uuid::Uuid, DndPlayerPiece>,
}

enum Signal {
    Autosave,
}

#[derive(Error, Debug)]
enum ServerError {
    #[error("No board saves exist, starting fresh!")]
    NoBoardSaves,
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

        handler.network().listen(Transport::Ws, addr)?;

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

        server.load_latest_board();

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
                            self.broadcast_log_message(
                                endpoint,
                                User::server(),
                                LogMessage::Joined(name),
                            )
                        }
                        DndMessage::UnregisterUser(name) => {
                            self.unregister(&name);
                        }
                        DndMessage::UserNotificationRemoved(_) => todo!(),
                        DndMessage::Log(user, msg) => {
                            self.broadcast_log_message(endpoint, user, msg)
                        }
                        DndMessage::RetrieveCharacterData(user) => {
                            match self.get_item_list(&user) {
                                Ok(list) => {
                                    let msg = DndMessage::ItemList(list);
                                    let encoded = bincode::serialize(&msg).unwrap();
                                    self.handler.network().send(endpoint, &encoded);
                                }
                                Err(e) => {
                                    error!("Failed to get item list for {}: {e:?}", user.name)
                                }
                            }

                            match self.get_ability_list(&user) {
                                Ok(list) => {
                                    let msg = DndMessage::AbilityList(list);
                                    let encoded = bincode::serialize(&msg).unwrap();
                                    self.handler.network().send(endpoint, &encoded);
                                }
                                Err(e) => {
                                    error!("Failed to get ability list for {}: {e:?}", user.name)
                                }
                            }

                            match self.get_character_stats(&user) {
                                Ok(stats) => {
                                    let msg = DndMessage::CharacterData(stats);
                                    let encoded = bincode::serialize(&msg).unwrap();
                                    self.handler.network().send(endpoint, &encoded);
                                }
                                Err(e) => {
                                    error!("Failed to get character stats for {}: {e:?}", user.name)
                                }
                            }

                            self.send_initial_board_data(endpoint);
                        }
                        DndMessage::UpdateItemCount(user, item_id, new_count) => {
                            self.update_item_count(user, item_id, new_count)
                        }
                        DndMessage::UpdateAbilityCount(user, ability_name, count) => {
                            self.update_ability_count(user, ability_name, count)
                        }
                        DndMessage::UpdateSkills(user, skill_list) => {
                            self.update_skills(user, skill_list)
                        }
                        DndMessage::UpdateHealth(user, curr_health, max_health) => {
                            self.update_health(user, curr_health, max_health)
                        }
                        DndMessage::UpdatePowerSlotCount(user, count) => {
                            self.update_powerslot_count(user, count.into());
                        }
                        DndMessage::BoardMessage(msg) => self.handle_board_message(endpoint, msg),
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
                        self.broadcast_log_message(
                            endpoint,
                            User::server(),
                            LogMessage::Disconnected(name.clone()),
                        );
                        self.unregister(&name.clone());
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

    fn get_ability_list(&self, user: &User) -> Result<Vec<Ability>, Box<dyn Error>> {
        info!("Retrieving ability list for {}", user.name);
        let res = futures::executor::block_on(async {
            let resp = self
                .db
                .from("player_abilities")
                .select("abilities(*),uses")
                .eq("player", user.name.clone())
                .execute()
                .await
                .unwrap();
            resp.text().await
        })?;

        info!("{}", res);
        let abilities: Vec<DBAbilityResponse> = serde_json::from_str(&res)?;

        Ok(abilities.into_iter().map(|x| x.into()).collect())
    }

    fn get_item_list(&self, user: &User) -> Result<Vec<Item>, Box<dyn Error>> {
        info!("Retrieving item list for {}", user.name);
        let res = futures::executor::block_on(async {
            let resp = self
                .db
                .from("inventory")
                .select("count,items(*)")
                .eq("player", user.name.clone())
                .execute()
                .await
                .unwrap();
            resp.text().await
        })?;

        info!("{}'s items {}", user.name, res);
        let items: Vec<DBItemResponse> = serde_json::from_str(&res)?;

        Ok(items.into_iter().map(|x| x.into()).collect())
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

    fn update_item_count(&self, user: User, item_id: i64, new_count: u32) {
        if new_count > 0 {
            futures::executor::block_on(async {
                self.db
                    .from("inventory")
                    .eq("player", &user.name)
                    .eq("item_id", item_id.to_string())
                    .update(format!("{{ \"count\": {} }}", new_count))
                    .execute()
                    .await
                    .unwrap();
            });

            info!("{}'s item count updated to {}", user.name, new_count);
        } else {
            futures::executor::block_on(async {
                self.db
                    .from("inventory")
                    .eq("player", &user.name)
                    .eq("item_id", item_id.to_string())
                    .delete()
                    .execute()
                    .await
                    .unwrap();
            });

            info!("{}'s item count reached 0, deleting from DB", user.name);
        }
    }

    fn update_ability_count(&self, user: User, ability_name: String, new_count: i64) {
        futures::executor::block_on(async {
            self.db
                .from("player_abilities")
                .eq("player", &user.name)
                .eq("ability_name", ability_name)
                .update(format!("{{ \"uses\": {} }}", new_count))
                .execute()
                .await
                .unwrap();
        });

        info!("{}'s ability uses updated to {}", user.name, new_count);
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

    fn get_character_stats(&self, user: &User) -> Result<Character, Box<dyn Error>> {
        let res = futures::executor::block_on(async {
            let resp = self
                .db
                .from("character")
                .select("*")
                .eq("name", user.name.clone())
                .single()
                .execute()
                .await
                .unwrap();
            resp.text().await
        })?;

        info!("'{}' character data {res}", user.name);

        serde_json::from_str(&res).map_err(|e| e.into())
    }

    fn broadcast_log_message(&self, ignore_enpoint: Endpoint, username: User, msg: LogMessage) {
        info!("Broadcasting log message!");
        let message = DndMessage::Log(username, msg);
        let output_data = bincode::serialize(&message).unwrap();
        for (_name, user) in self.users.iter() {
            if user.endpoint != ignore_enpoint {
                self.handler.network().send(user.endpoint, &output_data);
            }
        }
    }

    fn handle_board_message(&mut self, from: Endpoint, msg: BoardMessage) {
        match msg.clone() {
            BoardMessage::AddPlayerPiece(uuid, player) => {
                self.board_data.players.insert(uuid, player);
            }
            BoardMessage::UpdatePlayerPiece(uuid, new_player) => {
                let Some(player) = self.board_data.players.get_mut(&uuid) else {
                    error!("Player {uuid} could not be found on the server!");
                    return;
                };

                *player = new_player;
            }
            BoardMessage::UpdatePlayerLocation(uuid, new_location) => {
                let Some(player) = self.board_data.players.get_mut(&uuid) else {
                    error!("Player {uuid} could not be found on the server!");
                    return;
                };

                player.position = new_location;
            }
            BoardMessage::DeletePlayerPiece(uuid) => {
                self.board_data.players.remove(&uuid);
            }
        }

        self.broadcast_board_message(from, msg);
    }

    fn send_initial_board_data(&self, endpoint: Endpoint) {
        for (uuid, player) in self.board_data.players.iter() {
            let message =
                DndMessage::BoardMessage(BoardMessage::AddPlayerPiece(*uuid, player.clone()));
            let output_data = bincode::serialize(&message).unwrap();
            self.handler.network().send(endpoint, &output_data);
        }
    }

    fn broadcast_board_message(&self, ignore_enpoint: Endpoint, msg: BoardMessage) {
        let message = DndMessage::BoardMessage(msg);
        let output_data = bincode::serialize(&message).unwrap();
        for (_name, user) in self.users.iter() {
            if user.endpoint != ignore_enpoint {
                self.handler.network().send(user.endpoint, &output_data);
            }
        }
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

    fn load_latest_board(&mut self) {
        match futures::executor::block_on(self.get_latest_board_data()) {
            Ok(data) => {
                self.board_data = data;
                info!("Loaded latest board data");
            }
            Err(e) => error!("Failed to load latest board: {e}"),
        };
    }

    async fn get_latest_board_data(&self) -> Result<BoardData, Box<dyn Error>> {
        let resp = self
            .db
            .from("board_data")
            .select("data")
            .order_with_options("created_at", None::<String>, false, false)
            .execute()
            .await?;

        #[derive(serde::Deserialize)]
        struct ServerData {
            data: BoardData,
        }

        let data = resp.text().await?;

        info!("Board data {data}");

        let all_saves: Vec<ServerData> = serde_json::from_str(&data)?;

        all_saves
            .into_iter()
            .next()
            .map(|x| x.data)
            .ok_or_else(|| ServerError::NoBoardSaves.into())
    }
}

use std::{
    collections::HashMap,
    error::Error,
    io,
    net::{SocketAddr, ToSocketAddrs},
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

pub struct DndServer {
    handler: NodeHandler<()>,
    board_data: BoardData,
    node_listener: Option<NodeListener<()>>,
    users: HashMap<String, ClientInfo>,
    db: Postgrest,
}

impl DndServer {
    pub fn new(addr: &str, port: u16) -> io::Result<Self> {
        let (handler, node_listener) = node::split::<()>();
        let addr = (addr, port).to_socket_addrs().unwrap().next().unwrap();

        handler.network().listen(Transport::Ws, addr)?;

        let url = dotenv::var("NEXT_PUBLIC_SUPABASE_URL").unwrap();
        let db = Postgrest::new(url).insert_header(
            "apikey",
            dotenv::var("NEXT_PUBLIC_SUPABASE_ANON_KEY").unwrap(),
        );

        info!("Connected to DB");

        info!("Server running at {}", addr);

        Ok(Self {
            db,
            handler,
            node_listener: Some(node_listener),
            users: HashMap::new(),
            board_data: BoardData::default(),
        })
    }

    pub fn run(mut self) {
        let node_listener = self.node_listener.take().unwrap();
        node_listener.for_each(move |event| match event.network() {
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
                    DndMessage::Log(user, msg) => self.broadcast_log_message(endpoint, user, msg),
                    DndMessage::RetrieveCharacterData(user) => {
                        match self.get_item_list(&user) {
                            Ok(list) => {
                                let msg = DndMessage::ItemList(list);
                                let encoded = bincode::serialize(&msg).unwrap();
                                self.handler.network().send(endpoint, &encoded);
                            }
                            Err(e) => error!("Failed to get item list for {}: {e:?}", user.name),
                        }

                        match self.get_ability_list(&user) {
                            Ok(list) => {
                                let msg = DndMessage::AbilityList(list);
                                let encoded = bincode::serialize(&msg).unwrap();
                                self.handler.network().send(endpoint, &encoded);
                            }
                            Err(e) => error!("Failed to get ability list for {}: {e:?}", user.name),
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
        });
    }

    fn register(&mut self, name: &str, endpoint: Endpoint) {
        if !self.users.contains_key(name) {
            let list = self.users.keys().cloned().collect();

            let message = DndMessage::UserList(list);
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

        //let res = futures::executor::block_on(async {
        //    let resp = self
        //        .db
        //        .from("character")
        //        .select("*,inventory(*, items(*))")
        //        .eq("name", &user.name)
        //        .execute()
        //        .await
        //        .unwrap();
        //    resp.text().await
        //})?;

        //info!("Test query resp {}", res);

        Ok(items.into_iter().map(|x| x.into()).collect())
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
}

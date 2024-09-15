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

use common::{message::DndMessage, Item, User};
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

pub struct DndServer {
    handler: NodeHandler<()>,
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
                    }
                    DndMessage::UnregisterUser(name) => {
                        self.unregister(&name);
                    }
                    DndMessage::UserNotificationRemoved(_) => todo!(),
                    DndMessage::Chat(user, msg) => self.broadcast_log_message(user, msg),
                    DndMessage::RetrieveItemList(user) => {
                        if let Ok(list) = self.get_item_list(user) {
                            let msg = DndMessage::ItemList(list);
                            let encoded = bincode::serialize(&msg).unwrap();
                            self.handler.network().send(endpoint, &encoded);
                        }
                    }
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
                    self.unregister(&name.clone())
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

    fn get_item_list(&self, user: User) -> Result<Vec<Item>, Box<dyn Error>> {
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

        let res = futures::executor::block_on(async {
            let resp = self
                .db
                .from("character")
                .select("*,inventory(*, items(*))")
                .eq("name", user.name.clone())
                .execute()
                .await
                .unwrap();
            resp.text().await
        })?;

        info!("Test query resp {}", res);

        Ok(items.into_iter().map(|x| x.into()).collect())
    }

    fn broadcast_log_message(&self, username: User, msg: String) {
        let message = DndMessage::Chat(username, msg);
        let output_data = bincode::serialize(&message).unwrap();
        for (_name, user) in self.users.iter() {
            self.handler.network().send(user.endpoint, &output_data);
        }
    }
}

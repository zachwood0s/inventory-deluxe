use std::{
    collections::HashMap,
    io,
    net::{SocketAddr, ToSocketAddrs},
};

use message_io::{
    network::{Endpoint, NetEvent, Transport},
    node::{self, NodeHandler, NodeListener},
};

use common::{message::DndMessage, User};

struct ClientInfo {
    user_data: User,
    endpoint: Endpoint,
}

fn main() -> io::Result<()> {
    let server = DndServer::new("0.0.0.0", 80)?;
    server.run();

    Ok(())
}

pub struct DndServer {
    handler: NodeHandler<()>,
    node_listener: Option<NodeListener<()>>,
    users: HashMap<String, ClientInfo>,
}

impl DndServer {
    pub fn new(addr: &str, port: u16) -> io::Result<Self> {
        let (handler, node_listener) = node::split::<()>();
        let addr = (addr, port).to_socket_addrs().unwrap().next().unwrap();

        handler.network().listen(Transport::Ws, addr)?;

        println!("Server running at {}", addr);

        Ok(Self {
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
                    _ => {
                        println!("Unhandled message {message:?}");
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

            println!("Added user '{}'", name);
        } else {
            println!(
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

            println!("Removed participant '{}'", name);
        } else {
            println!("Cannot unregister a user '{}' who doesn't exist??", name);
        }
    }

    fn broadcast_log_message(&self, username: User, msg: String) {
        let message = DndMessage::Chat(username, msg);
        let output_data = bincode::serialize(&message).unwrap();
        for (_name, user) in self.users.iter() {
            self.handler.network().send(user.endpoint, &output_data);
        }
    }
}

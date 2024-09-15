use std::net::SocketAddr;

use crate::{Item, User};

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum DndMessage {
    // Bidirectional
    Chat(User, String),

    // From Client
    RegisterUser(String),
    UnregisterUser(String),
    RetrieveItemList(User),

    // From DndServer
    UserList(Vec<String>),
    UserNotificationAdded(String),
    UserNotificationRemoved(String),
    ItemList(Vec<Item>),
}

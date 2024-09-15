use std::net::SocketAddr;

use crate::User;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum DndMessage {
    // Bidirectional
    Chat(User, String),

    // From Client
    RegisterUser(String),
    UnregisterUser(String),

    // From DndServer
    UserList(Vec<String>),
    UserNotificationAdded(String),
    UserNotificationRemoved(String),
}

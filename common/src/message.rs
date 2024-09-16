use std::net::SocketAddr;

use crate::{Character, Item, User};

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum LogMessage {
    Chat(String),
    UseItem(String, u32),
    Joined(String),
    Disconnected(String),
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum DndMessage {
    // Bidirectional
    Log(User, LogMessage),

    // From Client
    RegisterUser(String),
    UnregisterUser(String),
    RetrieveCharacterData(User),
    /// (User, id, new_count)
    UpdateItemCount(User, i64, u32),

    // From DndServer
    UserList(Vec<String>),
    UserNotificationAdded(String),
    UserNotificationRemoved(String),
    ItemList(Vec<Item>),
    CharacterData(Character),
}

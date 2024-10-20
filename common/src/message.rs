use std::net::SocketAddr;

use emath::Pos2;
use uuid::Uuid;

use crate::{Ability, Character, DndPlayerPiece, Item, User};

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum LogMessage {
    Chat(String),
    UseItem(String, u32),
    SetAbilityCount(String, i64),
    Joined(String),
    Disconnected(String),
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum BoardMessage {
    AddPlayerPiece(Uuid, DndPlayerPiece),
    UpdatePlayerLocation(Uuid, Pos2),
    DeletePlayerPiece(Uuid),
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
    UpdateAbilityCount(User, String, i64),

    // Board
    BoardMessage(BoardMessage),

    // From DndServer
    UserList(Vec<String>),
    UserNotificationAdded(String),
    UserNotificationRemoved(String),
    ItemList(Vec<Item>),
    CharacterData(Character),
    AbilityList(Vec<Ability>),
}

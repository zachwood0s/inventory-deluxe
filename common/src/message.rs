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
    Roll(u32, u32),
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum BoardMessage {
    AddPlayerPiece(Uuid, DndPlayerPiece),
    UpdatePlayerPiece(Uuid, DndPlayerPiece),
    UpdatePlayerLocation(Uuid, Pos2),
    DeletePlayerPiece(Uuid),
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
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
    UpdatePowerSlotCount(User, i16),

    UpdateSkills(User, Vec<String>),

    // Board
    BoardMessage(BoardMessage),

    // From DndServer
    UserList(Vec<String>),
    CharacterList(Vec<String>),
    UserNotificationAdded(String),
    UserNotificationRemoved(String),
    ItemList(Vec<Item>),
    CharacterData(Character),
    AbilityList(Vec<Ability>),
}

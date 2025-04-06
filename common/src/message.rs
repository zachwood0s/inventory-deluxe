use crate::{
    board::{BoardPiece, PieceId},
    Ability, Character, Item, User,
};

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Log {
    pub user: User,
    pub payload: LogMessage,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct SingleDieRoll {
    pub value: u32,
    pub taken: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct DieRoll {
    pub roll_str: String,
    pub total: u32,
    pub rolls: Vec<SingleDieRoll>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum LogMessage {
    Chat(String),
    UseItem(String, u32),
    SetAbilityCount(String, i64),
    Server(String),
    Joined(String),
    Disconnected(String),
    Roll(DieRoll),
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum BoardMessage {
    AddOrUpdatePiece(BoardPiece),
    DeletePiece(PieceId),
    StoreBackpackPiece(BackpackPiece),
    RemoveBackpackPiece(String),
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct SaveBoard {
    pub tag: Option<String>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct LoadBoard {
    pub tag: String,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct RegisterUser {
    pub name: String,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct UnRegisterUser {
    pub name: String,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct RetrieveCharacterData {
    pub user: User,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct UpdateAbilityCount {
    pub user: User,
    pub ability_name: String,
    pub new_count: i64,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct UpdateItemCount {
    pub user: User,
    pub item_id: i64,
    pub new_count: u32,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct UpdatePowerSlotCount {
    pub user: User,
    pub new_count: i16,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct BackpackPiece {
    pub user: User,
    pub category: String,
    pub piece: BoardPiece,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct UpdateSkills {
    pub user: User,
    pub skills: Vec<String>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct UpdateHealth {
    pub user: User,
    pub cur_health: i16,
    pub max_health: i16,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub enum DndMessage {
    // Bidirectional
    Log(Log),

    // From Client
    RegisterUser(RegisterUser),
    UnregisterUser(UnRegisterUser),
    RetrieveCharacterData(RetrieveCharacterData),
    UpdateItemCount(UpdateItemCount),
    UpdateAbilityCount(UpdateAbilityCount),
    UpdatePowerSlotCount(UpdatePowerSlotCount),

    // Character
    UpdateSkills(UpdateSkills),
    UpdateHealth(UpdateHealth),

    // Board
    BoardMessage(BoardMessage),
    SaveBoard(SaveBoard),
    LoadBoard(LoadBoard),

    // From DndServer
    UserList(Vec<String>),
    CharacterList(Vec<String>),
    UserNotificationAdded(String),
    UserNotificationRemoved(String),
    ItemList(Vec<Item>),
    CharacterData(Character),
    AbilityList(Vec<Ability>),
}

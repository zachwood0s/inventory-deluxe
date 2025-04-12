use crate::{
    board::{BoardMessage, BoardPiece, PieceId},
    data_store::DataMessage,
    Ability, Character, CharacterSemiStatic, CharacterStats, Item, ItemId, User,
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
#[deprecated]
pub struct RetrieveCharacterData {
    pub user: User,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
#[deprecated]
pub struct UpdateAbilityCount {
    pub user: User,
    pub ability_name: String,
    pub new_count: i64,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
#[deprecated]
pub struct UpdateItemCount {
    pub user: User,
    pub item_id: ItemId,
    pub new_count: u32,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
#[deprecated]
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
#[deprecated]
pub struct UpdateSkills {
    pub user: User,
    pub skills: Vec<String>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
#[deprecated]
pub struct UpdateCharacterStats {
    pub user: User,
    pub new_stats: CharacterStats,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub enum DndMessage {
    // Bidirectional
    Log(Log),

    // From Client
    RegisterUser(RegisterUser),
    UnregisterUser(UnRegisterUser),
    RetrieveCharacterData(RetrieveCharacterData),
    #[deprecated]
    UpdateItemCount(UpdateItemCount),
    #[deprecated]
    UpdateAbilityCount(UpdateAbilityCount),
    #[deprecated]
    UpdatePowerSlotCount(UpdatePowerSlotCount),

    // Character
    DataMessage(DataMessage),
    #[deprecated]
    UpdateCharacterStats(UpdateCharacterStats),
    #[deprecated]
    UpdateSkills(UpdateSkills),

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

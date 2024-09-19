use emath::{Pos2, Vec2};

pub mod message;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct User {
    pub name: String,
}

impl User {
    pub fn server() -> Self {
        Self {
            name: String::from("<<SERVER>>"),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Item {
    pub id: i64,
    pub count: u32,
    pub name: String,
    pub description: String,
    pub flavor_text: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
pub struct Character {
    pub name: String,
    pub int: i16,
    pub wis: i16,
    pub str: i16,
    pub chr: i16,
    pub dex: i16,
    pub con: i16,
    pub tagline: String,
    pub backstory: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
pub struct DndPlayerPiece {
    pub position: Pos2,
    pub size: Vec2,
    pub image_url: Option<String>,
    pub color: Option<[u8; 4]>,
}

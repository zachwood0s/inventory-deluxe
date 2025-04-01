use std::fmt::{Debug, Display};

use emath::{Pos2, Vec2};

pub mod board;
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

impl Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.name, f)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Item {
    pub id: i64,
    pub count: u32,
    pub name: String,
    pub description: String,
    pub flavor_text: String,
    pub quest_item: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Ability {
    pub name: String,
    pub description: String,
    pub notes: Option<String>,
    pub ability_type: String,
    pub flavor_text: Option<String>,
    pub resource: String,
    pub max_count: i64,
    pub uses: i64,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
pub struct Character {
    pub name: String,
    pub int: i16,
    pub wis: i16,
    pub str: i16,
    pub cha: i16,
    pub dex: i16,
    pub con: i16,
    pub tagline: String,
    pub backstory: String,
    pub skills: Vec<String>,
    pub power_slots: i16,
    pub max_hp: i16,
    pub curr_hp: i16,
}

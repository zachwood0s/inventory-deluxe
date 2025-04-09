use std::fmt::{Debug, Display};

use derive_more::{Deref, DerefMut};

pub mod board;
pub mod message;

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Clone,
    Default,
    Deref,
    DerefMut,
    derive_more::Into,
    derive_more::From,
    Hash,
    PartialEq,
    Eq,
)]
#[serde(from = "String")]
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

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Clone,
    Default,
    derive_more::Deref,
    derive_more::DerefMut,
)]
pub struct Character {
    pub info: CharacterSemiStatic,
    #[deref]
    #[deref_mut]
    pub stats: CharacterStats,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct CharacterStats {
    pub int: i16,
    pub wis: i16,
    pub str: i16,
    pub cha: i16,
    pub dex: i16,
    pub con: i16,
    pub power_slots: i16,
    pub max_hp: i16,
    pub curr_hp: i16,
}

impl CharacterStats {
    pub fn with_int(mut self, int: i16) -> Self {
        self.int = int;
        self
    }

    pub fn with_wis(mut self, wis: i16) -> Self {
        self.wis = wis;
        self
    }

    pub fn with_str(mut self, str: i16) -> Self {
        self.str = str;
        self
    }

    pub fn with_cha(mut self, cha: i16) -> Self {
        self.cha = cha;
        self
    }

    pub fn with_dex(mut self, dex: i16) -> Self {
        self.dex = dex;
        self
    }

    pub fn with_con(mut self, con: i16) -> Self {
        self.con = con;
        self
    }

    pub fn with_powerslots(mut self, power_slots: i16) -> Self {
        self.power_slots = power_slots;
        self
    }

    pub fn with_max_hp(mut self, max_hp: i16) -> Self {
        self.max_hp = max_hp;
        self
    }

    pub fn with_curr_hp(mut self, curr_hp: i16) -> Self {
        self.curr_hp = curr_hp;
        self
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
pub struct CharacterSemiStatic {
    pub name: User,
    pub tagline: String,
    pub backstory: String,
    pub skills: Vec<String>,
}

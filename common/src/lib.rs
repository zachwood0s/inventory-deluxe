use std::fmt::{Debug, Display};

use derive_more::{Deref, DerefMut};

pub mod board;
pub mod data_store;
pub mod message;

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Clone,
    Default,
    Deref,
    DerefMut,
    derive_more::Display,
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

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Clone,
    Default,
    Deref,
    DerefMut,
    derive_more::Display,
    derive_more::Into,
    derive_more::From,
    Hash,
    PartialEq,
    Eq,
)]
#[serde(from = "String")]
pub struct AbilityId {
    pub name: String,
}

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Clone,
    Copy,
    Hash,
    Eq,
    PartialEq,
    derive_more::From,
    derive_more::Display,
)]
#[serde(transparent)]
pub struct ItemId(i64);

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Clone,
    Copy,
    Hash,
    Eq,
    PartialEq,
    PartialOrd,
    Ord,
    derive_more::Display,
)]
pub enum ItemCategory {
    Weapons,
    Equipment,
    Consumables,
    Valuables,
    Misc,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Item {
    pub id: ItemId,
    #[deprecated]
    #[serde(skip)]
    pub count: u32,
    pub name: String,
    pub description: String,
    pub flavor_text: Option<String>,
    pub quest_item: bool,
    pub equippable: bool,
    pub requires_attunement: bool,
    pub category: ItemCategory,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Ability {
    pub name: AbilityId,
    pub description: String,
    pub notes: Option<String>,
    pub ability_type: String,
    pub flavor_text: Option<String>,
    pub resource: String,
    pub max_count: i64,
    #[deprecated]
    #[serde(skip)]
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

#[derive(Clone, Copy, PartialEq, Eq, Debug, derive_more::Display)]
#[allow(dead_code)]
pub enum CharStat {
    #[display("CHA")]
    Cha,
    #[display("STR")]
    Str,
    #[display("WIS")]
    Wis,
    #[display("INT")]
    Int,
    #[display("DEX")]
    Dex,
    #[display("CON")]
    Con,
    #[display("SPD")]
    Spd,
    #[display("AC")]
    Ac,
}

impl CharStat {
    pub const ALL: [CharStat; 8] = [
        CharStat::Str,
        CharStat::Dex,
        CharStat::Con,
        CharStat::Int,
        CharStat::Wis,
        CharStat::Cha,
        CharStat::Spd,
        CharStat::Ac,
    ];

    pub fn full_name(&self) -> &'static str {
        match self {
            CharStat::Cha => "CHARISMA",
            CharStat::Str => "STRENGTH",
            CharStat::Wis => "WISDOM",
            CharStat::Int => "INTELLIGENCE",
            CharStat::Dex => "DEXTERITY",
            CharStat::Con => "CONSTITUTION",
            CharStat::Spd => "SPEED",
            CharStat::Ac => "AC",
        }
    }

    pub fn has_modifier(&self) -> bool {
        match self {
            CharStat::Cha => true,
            CharStat::Str => true,
            CharStat::Wis => true,
            CharStat::Int => true,
            CharStat::Dex => true,
            CharStat::Con => true,
            CharStat::Spd => false,
            CharStat::Ac => false,
        }
    }
}

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Clone,
    Copy,
    Default,
    Eq,
    PartialEq,
    derive_more::Deref,
    derive_more::DerefMut,
)]
#[serde(from = "i16")]
pub struct StatValue(i16);

impl From<i16> for StatValue {
    fn from(value: i16) -> Self {
        StatValue(value)
    }
}

impl StatValue {
    pub fn mod_score(&self) -> i16 {
        (self.0 / 2) - 5
    }

    pub fn mod_string(&self) -> String {
        let modifier = self.mod_score();
        let prefix = if modifier > 0 { "+" } else { "" };
        format!("{prefix}{modifier}")
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct CharacterStats {
    pub int: StatValue,
    pub wis: StatValue,
    pub str: StatValue,
    pub cha: StatValue,
    pub dex: StatValue,
    pub con: StatValue,
    pub power_slots: i16,
    pub max_hp: i16,
    pub curr_hp: i16,
}

impl CharacterStats {
    pub fn get_stat(&self, stat: CharStat) -> StatValue {
        match stat {
            CharStat::Cha => self.cha,
            CharStat::Str => self.str,
            CharStat::Wis => self.wis,
            CharStat::Int => self.int,
            CharStat::Dex => self.dex,
            CharStat::Con => self.con,
            // TODO: Map to real stats
            CharStat::Spd => self.con,
            CharStat::Ac => self.con,
        }
    }

    pub fn get_stat_mut(&mut self, stat: CharStat) -> &mut StatValue {
        match stat {
            CharStat::Cha => &mut self.cha,
            CharStat::Str => &mut self.str,
            CharStat::Wis => &mut self.wis,
            CharStat::Int => &mut self.int,
            CharStat::Dex => &mut self.dex,
            CharStat::Con => &mut self.con,
            // TODO: Map to real stats
            CharStat::Spd => &mut self.dex,
            CharStat::Ac => &mut self.con,
        }
    }

    pub fn with_int(mut self, int: impl Into<StatValue>) -> Self {
        self.int = int.into();
        self
    }

    pub fn with_wis(mut self, wis: impl Into<StatValue>) -> Self {
        self.wis = wis.into();
        self
    }

    pub fn with_str(mut self, str: impl Into<StatValue>) -> Self {
        self.str = str.into();
        self
    }

    pub fn with_cha(mut self, cha: impl Into<StatValue>) -> Self {
        self.cha = cha.into();
        self
    }

    pub fn with_dex(mut self, dex: impl Into<StatValue>) -> Self {
        self.dex = dex.into();
        self
    }

    pub fn with_con(mut self, con: impl Into<StatValue>) -> Self {
        self.con = con.into();
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

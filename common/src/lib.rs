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
    pub backstory: String,
}


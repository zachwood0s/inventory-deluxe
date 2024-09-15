pub mod message;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct User {
    pub name: String,
}


use common::{Ability, Item};

#[derive(serde::Deserialize, Clone)]
pub struct DBItem {
    id: i64,
    name: String,
    description: String,
    flavor_text: String,
    quest_item: bool,
}

#[derive(serde::Deserialize, Clone)]
pub struct DBItemResponse {
    count: u32,
    items: DBItem,
}

#[allow(clippy::from_over_into)]
impl Into<common::Item> for DBItemResponse {
    fn into(self) -> common::Item {
        Item {
            id: self.items.id,
            count: self.count,
            name: self.items.name,
            description: self.items.description,
            flavor_text: self.items.flavor_text,
            quest_item: self.items.quest_item,
        }
    }
}

#[derive(serde::Deserialize, Clone)]
pub struct DBAbilityResponse {
    pub abilities: Ability,
}
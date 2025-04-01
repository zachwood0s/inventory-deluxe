
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
pub struct DBAbility {
    name: String,
    description: String,
    notes: Option<String>,
    ability_type: String,
    flavor_text: Option<String>,
    resource: String,
    max_count: i64,
}

#[derive(serde::Deserialize, Clone)]
pub struct DBAbilityResponse {
    pub abilities: DBAbility,
    pub uses: i64,
}

#[allow(clippy::from_over_into)]
impl Into<common::Ability> for DBAbilityResponse {
    fn into(self) -> common::Ability {
        Ability {
            name: self.abilities.name,
            description: self.abilities.description,
            notes: self.abilities.notes,
            ability_type: self.abilities.ability_type,
            flavor_text: self.abilities.flavor_text,
            resource: self.abilities.resource,
            max_count: self.abilities.max_count,
            uses: self.uses,
        }
    }
}

pub trait InnerInto<T>: Sized {
    fn inner_into(self) -> T;
}

impl<T, U> InnerInto<Vec<U>> for Vec<T>
where
    T: Into<U>,
{
    fn inner_into(self) -> Vec<U> {
        self.into_iter().map(|x| x.into()).collect()
    }
}

use std::{any, collections::HashMap, hash};

use log::{debug, error, info};
use thiserror::Error;

use crate::{
    message::DndMessage, Ability, AbilityId, AbilitySource, Character, CharacterSemiStatic,
    CharacterStats, Item, ItemId, User,
};

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, derive_more::From)]
pub enum DataMessage {
    UpdateItemHandle(UpdateItemHandle),
    UpdateAbilityCount(UpdateAbilityCount),
    UpdateCharacterStats(UpdateCharacterStats),
    UpdateSkills(UpdateSkills),
    OverwriteAllData(DataStore),
    OverwriteAbility(Ability),
}

// TODO: Get rid of this once we use derive_more::Into on the DndMessage enum
impl From<DataMessage> for DndMessage {
    fn from(value: DataMessage) -> Self {
        DndMessage::DataMessage(value)
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct UpdateItemHandle {
    pub user: User,
    pub handle: ItemHandle,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct UpdateAbilityCount {
    pub user: User,
    pub ability_name: AbilityId,
    pub new_count: i64,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct UpdateCharacterStats {
    pub user: User,
    pub new_stats: CharacterStats,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct UpdateSkills {
    pub user: User,
    pub skills: Vec<String>,
}

#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize, Hash, PartialEq, Eq)]
pub struct ItemHandle {
    pub item: ItemId,
    pub count: u32,
    pub equipped: bool,
    pub attuned: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct ItemRef<'a> {
    pub handle: ItemHandle,
    pub item: &'a Item,
}

impl hash::Hash for ItemRef<'_> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.handle.hash(state);
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, Hash, PartialEq, Eq)]
pub struct AbilityHandle {
    pub ability_name: AbilityId,
    pub uses: i64,
    #[serde(default)]
    pub ability_source: Option<AbilitySource>,
}

#[derive(Copy, Clone, Debug)]
pub struct AbilityRef<'a> {
    pub handle: &'a AbilityHandle,
    pub ability: &'a Ability,
}

impl hash::Hash for AbilityRef<'_> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.handle.hash(state);
    }
}

#[derive(Error, Debug)]
pub enum DataStoreError {
    #[error("Character with name {0} not found")]
    CharacterNotFound(User),
    #[error("Character {0} does not have ability {1}")]
    CharacterDoesNotHaveAbility(User, AbilityId),
    #[error("Character {0} does not have item {1}")]
    CharacterDoesNotHaveItem(User, ItemId),
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct CharacterStorage {
    data: Character,
    items: HashMap<ItemId, ItemHandle>,
    abilities: HashMap<AbilityId, AbilityHandle>,
}

impl CharacterStorage {
    fn update_granted_items(&mut self, items: &HashMap<ItemId, Item>) {
        // Keep only the abilities which are still granted
        // (i.e. we still have the item which is granting it to us)
        self.abilities
            .retain(|_, handle| match handle.ability_source {
                Some(AbilitySource::Item(item_id)) => self.items.contains_key(&item_id),
                None => true,
            });

        // Add in any abilities that we're now granted but don't have already
        for item_handle in self.items.values() {
            let Some(item) = items.get(&item_handle.item) else {
                continue;
            };

            for ability in item.granted_abilities() {
                if self.abilities.contains_key(ability) {
                    continue;
                };

                info!("Granted {ability}");

                self.abilities.insert(
                    ability.clone(),
                    AbilityHandle {
                        ability_name: ability.clone(),
                        uses: 1,
                        ability_source: Some(AbilitySource::Item(item_handle.item)),
                    },
                );
            }
        }
    }
}

impl CharacterStorage {
    pub fn from_data(
        data: Character,
        items: Vec<ItemHandle>,
        abilities: Vec<AbilityHandle>,
    ) -> Self {
        let items = items.into_iter().map(|item| (item.item, item)).collect();
        let abilities = abilities
            .into_iter()
            .map(|ab| (ab.ability_name.clone(), ab))
            .collect();

        Self {
            data,
            items,
            abilities,
        }
    }

    pub fn get_ability_mut(&mut self, id: &AbilityId) -> anyhow::Result<&mut AbilityHandle> {
        self.abilities.get_mut(id).ok_or_else(|| {
            DataStoreError::CharacterDoesNotHaveAbility(self.data.info.name.clone(), id.clone())
                .into()
        })
    }

    pub fn get_ability(&self, id: &AbilityId) -> anyhow::Result<&AbilityHandle> {
        self.abilities.get(id).ok_or_else(|| {
            DataStoreError::CharacterDoesNotHaveAbility(self.data.info.name.clone(), id.clone())
                .into()
        })
    }

    pub fn is_ability_active(&self, id: &AbilityId) -> bool {
        let Some(ability) = self.abilities.get(id) else {
            return false;
        };

        match ability.ability_source {
            Some(AbilitySource::Item(item_id)) => self
                .items
                .get(&item_id)
                .map(|item| item.equipped)
                .unwrap_or_default(),
            None => true,
        }
    }

    pub fn get_item(&self, id: &ItemId) -> anyhow::Result<&ItemHandle> {
        self.items.get(id).ok_or_else(|| {
            DataStoreError::CharacterDoesNotHaveItem(self.data.info.name.clone(), *id).into()
        })
    }

    pub fn get_item_mut(&mut self, id: &ItemId) -> anyhow::Result<&mut ItemHandle> {
        self.items.get_mut(id).ok_or_else(|| {
            DataStoreError::CharacterDoesNotHaveItem(self.data.info.name.clone(), *id).into()
        })
    }

    pub fn items<'a>(&'a self, data_store: &'a DataStore) -> impl Iterator<Item = ItemRef<'a>> {
        self.items.values().flat_map(|&handle| {
            data_store
                .get_item(&handle.item)
                .map(|item| ItemRef { handle, item })
        })
    }

    pub fn abilities<'a>(
        &'a self,
        data_store: &'a DataStore,
    ) -> impl Iterator<Item = AbilityRef<'a>> {
        self.abilities.values().flat_map(|handle| {
            data_store
                .get_ability(&handle.ability_name)
                .map(|ability| AbilityRef { handle, ability })
        })
    }

    pub fn stats(&self) -> &CharacterStats {
        &self.data.stats
    }

    pub fn info(&self) -> &CharacterSemiStatic {
        &self.data.info
    }

    pub fn name(&self) -> &User {
        &self.data.info.name
    }
}

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct DataStore {
    characters: HashMap<User, CharacterStorage>,
    items: HashMap<ItemId, Item>,
    abilities: HashMap<AbilityId, Ability>,
}

impl DataStore {
    pub fn handle_message(&mut self, msg: DataMessage) {
        debug!("Handling message: {msg:?}");

        let res = match msg {
            DataMessage::UpdateItemHandle(msg) => self.update_item_handle(msg),
            DataMessage::UpdateAbilityCount(msg) => self.update_ability_count(msg),
            DataMessage::UpdateCharacterStats(msg) => self.update_character_stats(msg),
            DataMessage::UpdateSkills(msg) => self.update_skills(msg),
            DataMessage::OverwriteAllData(msg) => self.overwrite_all_data(msg),
            DataMessage::OverwriteAbility(msg) => self.overwrite_ability(msg),
        };

        if let Err(err) = res {
            error!("Error handling DataStore message: {err}");
        }
    }

    fn update_item_handle(&mut self, msg: UpdateItemHandle) -> anyhow::Result<()> {
        let character = self.get_character_mut(&msg.user)?;
        let item_handle = character.get_item_mut(&msg.handle.item)?;
        *item_handle = msg.handle;
        Ok(())
    }

    fn update_ability_count(&mut self, msg: UpdateAbilityCount) -> anyhow::Result<()> {
        let character = self.get_character_mut(&msg.user)?;
        let ability = character.get_ability_mut(&msg.ability_name)?;
        ability.uses = msg.new_count;
        Ok(())
    }

    fn update_character_stats(&mut self, msg: UpdateCharacterStats) -> anyhow::Result<()> {
        let character = self.get_character_mut(&msg.user)?;
        character.data.stats = msg.new_stats;
        Ok(())
    }

    fn update_skills(&mut self, msg: UpdateSkills) -> anyhow::Result<()> {
        let character = self.get_character_mut(&msg.user)?;
        character.data.info.skills = msg.skills;
        Ok(())
    }

    fn overwrite_all_data(&mut self, new_data: DataStore) -> anyhow::Result<()> {
        *self = new_data;

        self.update_granted_items();

        Ok(())
    }

    fn overwrite_ability(&mut self, msg: Ability) -> anyhow::Result<()> {
        todo!();
    }

    pub fn overwrite_items(&mut self, new_items: Vec<Item>) {
        self.items = new_items.into_iter().map(|i| (i.id, i)).collect();

        self.update_granted_items();
    }

    pub fn overwrite_abilities(&mut self, new_abilities: Vec<Ability>) {
        self.abilities = new_abilities
            .into_iter()
            .map(|a| (a.name.clone(), a))
            .collect();
    }

    pub fn overwrite_characters(&mut self, new_characters: Vec<CharacterStorage>) {
        self.characters = new_characters
            .into_iter()
            .map(|c| (c.data.info.name.clone(), c))
            .collect();

        self.update_granted_items();
    }

    fn update_granted_items(&mut self) {
        for character in self.characters.values_mut() {
            character.update_granted_items(&self.items);
        }
    }
}

impl DataStore {
    pub fn get_character(&self, user: &User) -> Option<&CharacterStorage> {
        self.characters.get(user)
    }

    pub fn get_character_mut(&mut self, user: &User) -> anyhow::Result<&mut CharacterStorage> {
        self.characters
            .get_mut(user)
            .ok_or_else(|| DataStoreError::CharacterNotFound(user.clone()).into())
    }

    pub fn character_names(&self) -> impl Iterator<Item = &User> {
        self.characters.keys()
    }

    pub fn get_item(&self, id: &ItemId) -> Option<&Item> {
        self.items.get(id)
    }

    pub fn get_ability(&self, id: &AbilityId) -> Option<&Ability> {
        self.abilities.get(id)
    }

    pub fn get_ability_mut(&mut self, id: &AbilityId) -> Option<&mut Ability> {
        todo!();
    }
}

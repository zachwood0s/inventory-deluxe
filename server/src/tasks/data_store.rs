use std::{collections::HashMap, sync::Arc};

use anyhow::Context;
use common::{
    data_store::{AbilityHandle, CharacterStorage, DataMessage, DataStore, ItemHandle},
    Ability, AbilityId, Character, CharacterSemiStatic, CharacterStats, Item, ItemId, User,
};
use itertools::Itertools;
use log::info;
use tokio::{sync::RwLock, try_join};

use crate::{DndEndpoint, DndServer, ListenerCtx, ResponseTextWithError, ServerError};

use super::{Broadcast, Response, ReturnToSender, ServerTask};

#[derive(Default)]
pub struct ServerDataStore {
    data: Arc<RwLock<DataStore>>,
}

impl ServerDataStore {
    async fn process_message(&self, msg: DataMessage) -> Result<(), ServerError> {
        let mut data = self.data.write().await;
        data.handle_message(msg);

        Ok(())
    }

    pub(crate) async fn data_mut<F>(&self, updater: F)
    where
        F: FnOnce(&mut DataStore),
    {
        let mut data = self.data.write().await;

        updater(&mut data);
    }

    async fn cloned(&self) -> DataStore {
        self.data.read().await.clone()
    }
}

impl Response for DataMessage {
    type Action = Broadcast;
    type ResponseData = DataMessage;

    async fn response(
        self,
        _: DndEndpoint,
        server: &DndServer,
    ) -> anyhow::Result<Self::ResponseData> {
        // Handle the message locally
        server.data_store.process_message(self.clone()).await?;

        // Return it to be broadcast to everyone else
        Ok(self)
    }
}

/// Sends the latest DB data to a User
pub struct SendLatestDbData;
impl Response for SendLatestDbData {
    type Action = ReturnToSender;
    type ResponseData = DataMessage;

    async fn response(
        self,
        endpoint: DndEndpoint,
        server: &DndServer,
    ) -> anyhow::Result<Self::ResponseData> {
        info!("Sending latest db data to {endpoint:?}");

        Ok(DataMessage::OverwriteAllData(
            server.data_store.cloned().await,
        ))
    }
}

/// Pulls the latest DB data
///
/// Will spin off multiple tasks to grab the individual pieces from the various tables
pub struct PullLatestDbData;
impl Response for PullLatestDbData {
    type Action = Broadcast;
    type ResponseData = DataMessage;

    async fn response(
        self,
        _: DndEndpoint,
        server: &DndServer,
    ) -> anyhow::Result<Self::ResponseData> {
        let items = self.pull_items(server);
        let abilities = self.pull_abilities(server);
        let characters = self.pull_characters(server);

        let (items, abilities, characters) = try_join!(items, abilities, characters)?;

        info!("Successfully pulled DB data");

        server
            .data_store
            .data_mut(|store| {
                store.overwrite_items(items);
                store.overwrite_abilities(abilities);
                store.overwrite_characters(characters);
            })
            .await;

        Ok(DataMessage::OverwriteAllData(
            server.data_store.cloned().await,
        ))
    }
}

impl PullLatestDbData {
    async fn pull_items(&self, server: &DndServer) -> anyhow::Result<Vec<Item>> {
        info!("Pulling item list");

        let raw_item = server
            .db
            .from("items")
            .select("*")
            .execute()
            .await?
            .text_with_error()
            .await?;

        serde_json::from_str(&raw_item).with_context(|| "Failed to parse items list")
    }

    async fn pull_abilities(&self, server: &DndServer) -> anyhow::Result<Vec<Ability>> {
        info!("Pulling ability list");

        let raw_ability = server
            .db
            .from("abilities")
            .select("*")
            .execute()
            .await?
            .text_with_error()
            .await?;

        serde_json::from_str(&raw_ability).with_context(|| "Failed to parse ability list")
    }

    async fn pull_characters(&self, server: &DndServer) -> anyhow::Result<Vec<CharacterStorage>> {
        info!("Pulling character list");

        async fn load_characters(server: &DndServer) -> anyhow::Result<Vec<Character>> {
            info!("Loading characters");

            let raw_character = server
                .db
                .from("character")
                .select("*")
                .execute()
                .await?
                .text_with_error()
                .await?;

            #[derive(serde::Deserialize)]
            struct DbCharacter {
                #[serde(flatten)]
                info: CharacterSemiStatic,
                #[serde(flatten)]
                stats: CharacterStats,
            }

            let characters: Vec<DbCharacter> = serde_json::from_str(&raw_character)
                .with_context(|| "Failed to parse character list")?;

            let characters = characters
                .into_iter()
                .map(|DbCharacter { info, stats }| Character { info, stats })
                .collect();

            Ok(characters)
        }

        async fn load_inventory_mapping(
            server: &DndServer,
        ) -> anyhow::Result<HashMap<User, Vec<ItemHandle>>> {
            info!("Loading inventories");

            let raw_inventory = server
                .db
                .from("inventory")
                .select("*")
                .execute()
                .await?
                .text_with_error()
                .await?;

            #[derive(serde::Deserialize)]
            #[allow(unused)]
            struct Item {
                id: i64,
                player: User,
                item_id: ItemId,
                count: u32,
                equipped: bool,
                attuned: bool,
            }

            let inventories: Vec<Item> = serde_json::from_str(&raw_inventory)
                .with_context(|| "Failed to parse inventory list")?;

            let mapping = inventories
                .into_iter()
                .map(|data| {
                    (
                        data.player,
                        ItemHandle {
                            item: data.item_id,
                            count: data.count,
                            equipped: data.equipped,
                            attuned: data.attuned,
                        },
                    )
                })
                .into_group_map();

            Ok(mapping)
        }

        async fn load_ability_mapping(
            server: &DndServer,
        ) -> anyhow::Result<HashMap<User, Vec<AbilityHandle>>> {
            info!("Loading abilities");

            let raw_abilities = server
                .db
                .from("player_abilities")
                .select("*")
                .execute()
                .await?
                .text_with_error()
                .await?;

            #[derive(serde::Deserialize)]
            #[allow(unused)]
            struct Ability {
                id: i64,
                player: User,
                ability_name: AbilityId,
                uses: i64,
            }

            let abilities: Vec<Ability> = serde_json::from_str(&raw_abilities)
                .with_context(|| "Failedto parse player ability list")?;

            let mapping = abilities
                .into_iter()
                .map(|data| {
                    (
                        data.player,
                        AbilityHandle {
                            ability_name: data.ability_name,
                            ability_source: None,
                            uses: data.uses,
                        },
                    )
                })
                .into_group_map();

            Ok(mapping)
        }

        let characters = load_characters(server);
        let inventories = load_inventory_mapping(server);
        let abilities = load_ability_mapping(server);

        let (characters, mut inventory, mut abilities) =
            try_join!(characters, inventories, abilities)?;

        let built = characters
            .into_iter()
            .map(|character| {
                let name = &character.info.name;
                let items = inventory.remove(name).unwrap_or_default();
                let abilities = abilities.remove(name).unwrap_or_default();

                CharacterStorage::from_data(character, items, abilities)
            })
            .collect();

        Ok(built)
    }
}

/// Writes the DB data
///
/// If the specified user is None, the whole DB is written back
pub struct WriteBackDbData(pub Option<User>);
impl ServerTask for WriteBackDbData {
    async fn process(
        self,
        endpoint: DndEndpoint,
        server: &DndServer,
        ctx: &ListenerCtx,
    ) -> anyhow::Result<()> {
        todo!()
    }
}

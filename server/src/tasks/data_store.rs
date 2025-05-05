use std::{collections::HashMap, sync::Arc};

use anyhow::Context;
use common::{
    data_store::{AbilityHandle, CharacterStorage, DataMessage, DataStore, ItemHandle},
    Ability, AbilityId, Character, CharacterSemiStatic, CharacterStats, Item, ItemId, User,
};
use futures::future::try_join;
use itertools::Itertools;
use log::{error, info, warn};
use tokio::{sync::RwLock, try_join};

use crate::{DndEndpoint, DndServer, ListenerCtx, Postgrest, ResponseTextWithError, ServerError};

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
                            db_id: data.id,
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
                            db_id: Some(data.id),
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
pub struct WriteBackDbData<'a>(pub Option<&'a User>);
impl ServerTask for WriteBackDbData<'_> {
    async fn process(
        self,
        _: DndEndpoint,
        server: &DndServer,
        _: &ListenerCtx,
    ) -> anyhow::Result<()> {
        let whole_db = self.0.is_none();
        let data_store = server.data_store.data.read().await;

        let users_to_write = match self.0 {
            Some(user) => vec![user],
            None => data_store.character_names().collect(),
        };

        WriteBackDbData::write_users(&data_store, &server.db, users_to_write).await?;

        // If writing whole DB, then also update items and abilities
        if whole_db {
            let abilities = WriteBackDbData::write_abilities(&data_store, &server.db);
            let items = WriteBackDbData::write_items(&data_store, &server.db);

            try_join!(abilities, items)?;
        }

        Ok(())
    }
}

impl WriteBackDbData<'_> {
    async fn write_users(
        data_store: &DataStore,
        db: &Postgrest,
        users: Vec<&User>,
    ) -> anyhow::Result<()> {
        async fn write_stats(
            _: &DataStore,
            db: &Postgrest,
            character: &CharacterStorage,
        ) -> anyhow::Result<()> {
            #[derive(serde::Serialize)]
            struct DbCharacter<'a> {
                #[serde(flatten)]
                info: &'a CharacterSemiStatic,
                #[serde(flatten)]
                stats: &'a CharacterStats,
            }

            let db_character = DbCharacter {
                info: character.info(),
                stats: character.stats(),
            };

            let serialized = serde_json::to_string(&db_character)?;

            db.from("character")
                .eq("name", &character.info().name.name)
                .update(serialized)
                .execute()
                .await?;

            info!("Saved {}'s stats", character.info().name);

            Ok(())
        }

        async fn write_inventory(
            data_store: &DataStore,
            db: &Postgrest,
            character: &CharacterStorage,
        ) -> anyhow::Result<()> {
            #[derive(serde::Serialize)]
            struct Item<'a> {
                id: i64,
                player: &'a User,
                item_id: ItemId,
                count: u32,
                equipped: bool,
                attuned: bool,
            }

            let db_items = character
                .items(data_store)
                .map(|item| Item {
                    id: item.handle.db_id,
                    player: character.name(),
                    item_id: item.handle.item,
                    count: item.handle.count,
                    equipped: item.handle.equipped,
                    attuned: item.handle.attuned,
                })
                .collect_vec();

            let serialized = serde_json::to_string(&db_items)?;

            db.from("inventory").upsert(serialized).execute().await?;

            info!("Saved {}'s inventory", character.info().name);

            Ok(())
        }

        async fn write_abilities(
            data_store: &DataStore,
            db: &Postgrest,
            character: &CharacterStorage,
        ) -> anyhow::Result<()> {
            #[derive(serde::Serialize)]
            struct Ability<'a> {
                id: i64,
                player: &'a User,
                ability_name: &'a AbilityId,
                uses: i64,
            }

            let db_items = character
                .abilities(data_store)
                .filter(|ability| ability.handle.db_id.is_some())
                .map(|ability| Ability {
                    // SAFE: we already filtered by only the "db" abilities
                    id: ability.handle.db_id.unwrap(),
                    player: character.name(),
                    ability_name: &ability.handle.ability_name,
                    uses: ability.handle.uses,
                })
                .collect_vec();

            let serialized = serde_json::to_string(&db_items)?;

            db.from("player_abilities")
                .upsert(serialized)
                .execute()
                .await?;

            info!("Saved {}'s abilities", character.info().name);

            Ok(())
        }

        // Write back one user at a time for now
        for user in users {
            let Some(character) = data_store.get_character(user) else {
                warn!("Tried to save non-existant character: {user}");
                continue;
            };

            let stats = write_stats(data_store, db, character);
            let inventory = write_inventory(data_store, db, character);
            let abilities = write_abilities(data_store, db, character);

            if let Err(e) = try_join!(stats, inventory, abilities) {
                error!("Failed to save user data: {e}");
            }

            info!("Saved {}", user);
        }

        Ok(())
    }

    async fn write_abilities(data_store: &DataStore, db: &Postgrest) -> anyhow::Result<()> {
        let abilities = data_store.abilities().collect_vec();
        let serialized = serde_json::to_string(&abilities)?;

        db.from("abilities").upsert(serialized).execute().await?;

        info!("Saved all abilities");

        Ok(())
    }

    async fn write_items(data_store: &DataStore, db: &Postgrest) -> anyhow::Result<()> {
        let items = data_store.items().collect_vec();
        let serialized = serde_json::to_string(&items)?;

        db.from("items").upsert(serialized).execute().await?;

        info!("Saved all items");

        Ok(())
    }
}

use anyhow::Context;
use common::{message::*, CharacterSemiStatic, CharacterStats, User};
use log::{debug, info};

use crate::{
    DBAbilityResponse, DBItemResponse, DndEndpoint, DndServer, InnerInto, ListenerCtx,
    ResponseTextWithError, ToError,
};

use super::{Broadcast, Response, ReturnToSender, ServerTask};

// TODO: The more I think about it the more I think its dumb to be interfacing directly with the DB
// this much. I think I should have a layer between, some DB manager, which pulls in all the data
// on start and also can save all data in the end or when a user drops. That way I can freely
// pound the server with requests but none are hitting the DB so we good. Then periodically or upon
// exit or upon disconnect I can trigger a DB save
// Process would be:
// 1. on boot pull down full DB of data (who gives a shit)
// 2. recieve requests from users, send from in memory
// 2a. server tasks would be super simple then, just a DB manager query for the most part
// 3. on disconnect/autosave/shutdown send data back to DB

/// Retrieves all of the available characters in the DB
pub struct GetCharacterList;
impl Response for GetCharacterList {
    type Action = ReturnToSender;
    type ResponseData = DndMessage;

    async fn response(
        self,
        _: DndEndpoint,
        server: &DndServer,
    ) -> anyhow::Result<Self::ResponseData> {
        info!("Retrieving character list");
        let resp = server.db.from("character").select("name").execute().await?;
        let chr_list = resp.text_with_error().await?;

        #[derive(serde::Deserialize)]
        struct Name {
            name: String,
        }

        let names: Vec<Name> = serde_json::from_str(&chr_list)?;
        Ok(DndMessage::CharacterList(
            names.into_iter().map(|x| x.name).collect(),
        ))
    }
}

/// Retrieves the item list for a given User and
/// sends them back to the endpoint which requested the list
pub struct GetItemList<'a>(pub &'a User);
impl Response for GetItemList<'_> {
    type Action = ReturnToSender;
    type ResponseData = DndMessage;

    async fn response(
        self,
        _: DndEndpoint,
        server: &DndServer,
    ) -> anyhow::Result<Self::ResponseData> {
        let Self(user) = self;

        info!("Retrieving item list for {}", user.name);
        let resp = server
            .db
            .from("inventory")
            .select("count,items(*)")
            .eq("player", user.name.clone())
            .execute()
            .await
            .with_context(|| format!("Failed to retrieve items from the DB for {user}"))?;

        let items = resp.text_with_error().await?;

        debug!("{}'s items {}", user.name, items);

        let items: Vec<DBItemResponse> = serde_json::from_str(&items)
            .with_context(|| format!("Failed to parse items list for {user}"))?;

        Ok(DndMessage::ItemList(items.inner_into()))
    }
}

/// Retrieves the ability list for a given User and
/// sends them back to the endpoint which requested the list
pub struct GetAbilityList<'a>(pub &'a User);
impl Response for GetAbilityList<'_> {
    type Action = ReturnToSender;
    type ResponseData = DndMessage;

    async fn response(
        self,
        _: DndEndpoint,
        server: &DndServer,
    ) -> anyhow::Result<Self::ResponseData> {
        let Self(user) = self;

        info!("Retrieving ability list for {}", user.name);

        let resp = server
            .db
            .from("player_abilities")
            .select("abilities(*),uses")
            .eq("player", user.name.clone())
            .execute()
            .await?;

        let abilities = resp.text_with_error().await?;

        debug!("{}'s abilities {}", user.name, abilities);

        let abilities: Vec<DBAbilityResponse> = serde_json::from_str(&abilities)?;

        Ok(DndMessage::AbilityList(abilities.inner_into()))
    }
}

/// Retrieves the character stats for a given User and
/// sends them back to the endpoint which requested the stats
pub struct GetCharacterStats<'a>(pub &'a User);
impl Response for GetCharacterStats<'_> {
    type Action = ReturnToSender;
    type ResponseData = DndMessage;

    async fn response(
        self,
        _: DndEndpoint,
        server: &DndServer,
    ) -> anyhow::Result<Self::ResponseData> {
        let Self(user) = self;

        let resp = server
            .db
            .from("character")
            .select("*")
            .eq("name", user.name.clone())
            .single()
            .execute()
            .await?;

        let stats = resp.text_with_error().await?;

        debug!("{}'s character data {}", user.name, stats);

        #[derive(serde::Deserialize)]
        struct DbCharacter {
            #[serde(flatten)]
            info: CharacterSemiStatic,
            #[serde(flatten)]
            stats: CharacterStats,
        }

        let DbCharacter { info, stats } = serde_json::from_str(&stats)?;

        Ok(DndMessage::CharacterData(common::Character { info, stats }))
    }
}

/// Updates the number of items of the specified type the specified user
/// has in their inventory
impl ServerTask for UpdateItemCount {
    async fn process(
        self,
        _: DndEndpoint,
        server: &DndServer,
        _: &ListenerCtx,
    ) -> anyhow::Result<()> {
        let Self {
            user,
            item_id,
            new_count,
        } = self;

        let req = server
            .db
            .from("inventory")
            .eq("player", &user.name)
            .eq("item_id", item_id.to_string());

        let req = if new_count > 0 {
            info!("{}'s item count updated to {}", user.name, new_count);
            req.update(format!("{{ \"count\": {} }}", new_count))
        } else {
            info!("{}'s item count reached 0, deleting from DB", user.name);
            req.delete()
        };

        req.execute().await?;

        Ok(())
    }
}

/// Updates the remaining number of usages a user has of the specified ability
impl ServerTask for UpdateAbilityCount {
    async fn process(
        self,
        _: DndEndpoint,
        server: &DndServer,
        _: &ListenerCtx,
    ) -> anyhow::Result<()> {
        let Self {
            user,
            ability_name,
            new_count,
        } = self;

        server
            .db
            .from("player_abilities")
            .eq("player", &user.name)
            .eq("ability_name", ability_name)
            .update(format!("{{ \"uses\": {} }}", new_count))
            .execute()
            .await?;

        info!("{}'s ability uses updated to {}", user.name, new_count);

        Ok(())
    }
}

/// Updates a user's remaning power slot count
impl ServerTask for UpdatePowerSlotCount {
    async fn process(
        self,
        _: DndEndpoint,
        server: &DndServer,
        _: &ListenerCtx,
    ) -> anyhow::Result<()> {
        let Self { user, new_count } = self;

        server
            .db
            .from("characters")
            .eq("player", &user.name)
            .update(format!("{{ \"power_slots\": {} }}", new_count))
            .execute()
            .await?
            .to_error()?;

        Ok(())
    }
}

/// Updates a user's skills
impl ServerTask for UpdateSkills {
    async fn process(
        self,
        _: DndEndpoint,
        server: &DndServer,
        _: &ListenerCtx,
    ) -> anyhow::Result<()> {
        let Self { user, skills } = self;
        let skill_vec = serde_json::to_string(&skills)?;

        server
            .db
            .from("character")
            .eq("name", &user.name)
            .update(format!("{{ \"skills\": {} }}", skill_vec))
            .execute()
            .await?
            .to_error()?;

        info!("{}'s skills updated to {}", &user.name, skill_vec);

        Ok(())
    }
}

impl ServerTask for UpdateCharacterStats {
    async fn process(
        self,
        endpoint: DndEndpoint,
        server: &DndServer,
        ctx: &ListenerCtx,
    ) -> anyhow::Result<()> {
        let Self { user, new_stats } = &self;

        let update = serde_json::to_string(new_stats)?;

        server
            .db
            .from("character")
            .eq("name", &user.name)
            .update(&update)
            .execute()
            .await?
            .to_error()?;

        info!("Updated {}'s character stats: {}", user.name, update);

        BroadcastCharacterStatsMessage(self)
            .process(endpoint, server, ctx)
            .await
    }
}

pub struct BroadcastCharacterStatsMessage(UpdateCharacterStats);
impl Response for BroadcastCharacterStatsMessage {
    type Action = Broadcast;
    type ResponseData = DndMessage;

    async fn response(
        self,
        _: DndEndpoint,
        _: &crate::DndServer,
    ) -> anyhow::Result<Self::ResponseData> {
        let Self(msg) = self;
        Ok(DndMessage::UpdateCharacterStats(msg))
    }
}

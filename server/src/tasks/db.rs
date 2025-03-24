use anyhow::Context;
use common::{message::DndMessage, User};
use log::{debug, info};
use message_io::network::Endpoint;

use crate::{DBAbilityResponse, DBItemResponse, InnerInto, ResponseTextWithError, ToError};

use super::{Response, ReturnToSender, ServerTask};

pub struct GetItemList<'a>(pub &'a User);
impl Response for GetItemList<'_> {
    type Action = ReturnToSender;
    type ResponseData = DndMessage;

    async fn response(
        self,
        _: message_io::network::Endpoint,
        server: &crate::DndServer,
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

pub struct GetAbilityList<'a>(pub &'a User);
impl Response for GetAbilityList<'_> {
    type Action = ReturnToSender;
    type ResponseData = DndMessage;

    async fn response(
        self,
        _: message_io::network::Endpoint,
        server: &crate::DndServer,
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

pub struct GetCharacterStats<'a>(pub &'a User);
impl Response for GetCharacterStats<'_> {
    type Action = ReturnToSender;
    type ResponseData = DndMessage;

    async fn response(
        self,
        _: message_io::network::Endpoint,
        server: &crate::DndServer,
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

        let stats = serde_json::from_str(&stats)?;

        Ok(DndMessage::CharacterData(stats))
    }
}

pub struct UpdateItemCount<'a> {
    user: &'a User,
    item_id: i64,
    new_count: u32,
}

impl<'a> UpdateItemCount<'a> {
    pub fn new(user: &'a User, item_id: i64, new_count: u32) -> Self {
        Self {
            user,
            item_id,
            new_count,
        }
    }
}

impl ServerTask for UpdateItemCount<'_> {
    async fn process(self, _: Endpoint, server: &mut crate::DndServer) -> anyhow::Result<()> {
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

pub struct UpdateAbilityCount<'a> {
    user: &'a User,
    ability_name: &'a String,
    new_count: i64,
}

impl<'a> UpdateAbilityCount<'a> {
    pub fn new(user: &'a User, ability_name: &'a String, new_count: i64) -> Self {
        Self {
            user,
            ability_name,
            new_count,
        }
    }
}

impl ServerTask for UpdateAbilityCount<'_> {
    async fn process(self, _: Endpoint, server: &mut crate::DndServer) -> anyhow::Result<()> {
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

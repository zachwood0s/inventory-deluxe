use common::message::{BoardMessage, DndMessage};
use log::info;
use message_io::network::Endpoint;

use crate::{BoardData, PlayerLookup, ResponseTextWithError, ServerError, ToError};

use super::{Broadcast, Response, ServerTask};

pub struct BroadcastBoardMessage(BoardMessage);
impl Response for BroadcastBoardMessage {
    type Action = Broadcast;
    type ResponseData = DndMessage;

    async fn response(
        self,
        _: Endpoint,
        _: &crate::DndServer,
    ) -> anyhow::Result<Self::ResponseData> {
        let Self(msg) = self;
        Ok(DndMessage::BoardMessage(msg))
    }
}

impl ServerTask for BoardMessage {
    async fn process(self, from: Endpoint, server: &mut crate::DndServer) -> anyhow::Result<()> {
        match self.clone() {
            BoardMessage::AddPlayerPiece(uuid, player) => {
                server.board_data.insert_player(uuid, player);
            }
            BoardMessage::UpdatePlayerPiece(uuid, new_player) => {
                let player = server.board_data.get_player_mut(&uuid)?;
                *player = new_player;
            }
            BoardMessage::UpdatePlayerLocation(uuid, new_location) => {
                let player = server.board_data.get_player_mut(&uuid)?;
                player.position = new_location;
            }
            BoardMessage::DeletePlayerPiece(uuid) => {
                server.board_data.remove_player(&uuid);
            }
        }

        BroadcastBoardMessage(self).process(from, server).await
    }
}

pub struct SendInitialBoardData;
impl ServerTask for SendInitialBoardData {
    async fn process(
        self,
        endpoint: Endpoint,
        server: &mut crate::DndServer,
    ) -> anyhow::Result<()> {
        for (uuid, player) in server.board_data.players.iter() {
            let message =
                DndMessage::BoardMessage(BoardMessage::AddPlayerPiece(*uuid, player.clone()));

            let output_data = bincode::serialize(&message)?;

            server
                .handler
                .network()
                .send(endpoint, &output_data)
                .to_error()?;
        }

        Ok(())
    }
}

pub struct GetLatestBoardData;
impl ServerTask for GetLatestBoardData {
    async fn process(self, _: Endpoint, server: &mut crate::DndServer) -> anyhow::Result<()> {
        let resp = server
            .db
            .from("board_data")
            .select("data")
            .order_with_options("created_at", None::<String>, false, false)
            .execute()
            .await?;

        #[derive(serde::Deserialize)]
        struct ServerData {
            data: BoardData,
        }

        let data = resp.text_with_error().await?;

        info!("Latest board data {data}");

        let all_saves: Vec<ServerData> = serde_json::from_str(&data)?;
        let board_data = all_saves
            .into_iter()
            .next()
            .map(|x| x.data)
            .ok_or(ServerError::NoBoardSaves)?;

        server.board_data = board_data;

        Ok(())
    }
}

pub struct SaveBoardData;
impl ServerTask for SaveBoardData {
    async fn process(self, _: Endpoint, server: &mut crate::DndServer) -> anyhow::Result<()> {
        if !server.board_data.is_dirty() {
            info!("Skipping autosave, no board modifications to save");
            return Ok(());
        }

        let json_board_data = serde_json::to_string(&server.board_data)?;

        server
            .db
            .from("board_data")
            .insert(format!(
                "{{\"data\": {json_board_data}, \"tag\": \"autosave\" }}"
            ))
            .execute()
            .await?
            .to_error()?;

        server.board_data.mark_clean();

        Ok(())
    }
}

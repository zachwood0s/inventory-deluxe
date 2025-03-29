use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc, RwLock},
};

use common::{
    message::{BoardMessage, DndMessage},
    DndPlayerPiece,
};
use emath::Pos2;
use log::info;
use message_io::network::Endpoint;

use crate::{DndServer, PlayerLookup, ResponseTextWithError, ServerError, ToError};

use super::{Broadcast, Response, ServerTask};

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Default)]
pub struct BoardData {
    #[serde(skip)]
    dirty: Arc<AtomicBool>,
    players: Arc<RwLock<PlayerLookup>>,
}

impl BoardData {
    fn process_message(&self, board_message: BoardMessage) -> Result<(), ServerError> {
        self.mark_dirty();
        let mut players = self.players.write().unwrap();

        match board_message {
            BoardMessage::AddPlayerPiece(uuid, player) => {
                players.insert(uuid, player);
            }
            BoardMessage::UpdatePlayerPiece(uuid, new_player) => {
                let player = players
                    .get_mut(&uuid)
                    .ok_or(ServerError::PlayerNotFound(uuid))?;

                *player = new_player;
            }
            BoardMessage::UpdatePlayerLocation(uuid, new_location) => {
                let player = players
                    .get_mut(&uuid)
                    .ok_or(ServerError::PlayerNotFound(uuid))?;

                player.position = new_location;
            }
            BoardMessage::DeletePlayerPiece(uuid) => {
                players.remove(&uuid);
            }
        }

        Ok(())
    }
    fn mark_dirty(&self) {
        self.dirty.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn mark_clean(&self) {
        self.dirty.store(false, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn get_players_owned(&self) -> HashMap<uuid::Uuid, DndPlayerPiece> {
        self.players.read().unwrap().clone()
    }

    pub fn overwrite_board_data(&self, new_data: BoardData) {
        let mut players = self.players.write().unwrap();
        *players = new_data.players.read().unwrap().clone();
    }
}

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
    async fn process(self, from: Endpoint, server: &DndServer) -> anyhow::Result<()> {
        server.board_data.process_message(self.clone())?;

        BroadcastBoardMessage(self).process(from, server).await
    }
}

pub struct SendInitialBoardData;
impl ServerTask for SendInitialBoardData {
    async fn process(self, endpoint: Endpoint, server: &DndServer) -> anyhow::Result<()> {
        for (uuid, player) in server.board_data.get_players_owned().into_iter() {
            let message = DndMessage::BoardMessage(BoardMessage::AddPlayerPiece(uuid, player));

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
    async fn process(self, _: Endpoint, server: &DndServer) -> anyhow::Result<()> {
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

        server.board_data.overwrite_board_data(board_data);

        Ok(())
    }
}

pub struct SaveBoardData;
impl ServerTask for SaveBoardData {
    async fn process(self, _: Endpoint, server: &DndServer) -> anyhow::Result<()> {
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

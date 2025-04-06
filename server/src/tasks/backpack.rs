use common::message::{BackpackMessage, StoreBackpackPiece};
use message_io::network::Endpoint;

use crate::{DndServer, ToError};

use super::ServerTask;

impl ServerTask for BackpackMessage {
    async fn process(self, endpoint: Endpoint, server: &DndServer) -> anyhow::Result<()> {
        match self {
            BackpackMessage::StoreBackpackPiece(store_player_piece) => {
                store_player_piece.process(endpoint, server).await
            }
            BackpackMessage::CreateBoardPiece() => todo!(),
        }
    }
}

impl ServerTask for StoreBackpackPiece {
    async fn process(self, _: Endpoint, server: &DndServer) -> anyhow::Result<()> {
        let piece_data = serde_json::to_string(&self.piece)?;

        server
            .db
            .from("backpack")
            .insert(format!(
                "{{\"owner\": \"{}\", \"category\": \"{}\", \"data\": {} }}",
                self.user.name, self.category, piece_data
            ))
            .execute()
            .await?
            .to_error()?;

        Ok(())
    }
}

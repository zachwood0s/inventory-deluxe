use std::sync::{atomic::AtomicBool, Arc, RwLock};

use common::message::{BoardMessage, DndMessage, LoadBoard, Log, LogMessage, SaveBoard};
use log::info;
use message_io::network::Endpoint;

use crate::{tasks::log::DirectMessage, DndServer, ResponseTextWithError, ServerError, ToError};

use super::{Broadcast, Response, ServerTask};

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct ServerBoardData {
    #[serde(skip)]
    dirty: Arc<AtomicBool>,
    board: Arc<RwLock<common::board::BoardData>>,
}

impl ServerBoardData {
    fn process_message(&self, board_message: BoardMessage) -> Result<(), ServerError> {
        self.mark_dirty();
        let mut board = self.board.write().unwrap();
        board.handle_message(board_message);

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

    pub fn overwrite_board_data(&self, new_data: common::board::BoardData) {
        let mut players = self.board.write().unwrap();
        *players = new_data;
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

impl ServerTask for SaveBoard {
    async fn process(self, _: Endpoint, server: &DndServer) -> anyhow::Result<()> {
        // Skip autosave if no tag is provided and no board modifications have happened
        if !server.board_data.is_dirty() && self.tag.is_none() {
            info!("Skipping autosave, no board modifications to save");
            return Ok(());
        }

        let json_board_data = serde_json::to_string(&server.board_data)?;

        let tag = self.tag.unwrap_or_else(|| String::from("autosave"));

        server
            .db
            .from("board_data")
            .insert(format!(
                "{{\"data\": {json_board_data}, \"tag\": \"{tag}\" }}"
            ))
            .execute()
            .await?
            .to_error()?;

        server.board_data.mark_clean();

        // Use self endpoint so it gets broadcasted to everyone
        Log {
            user: common::User::server(),
            payload: LogMessage::Server(format!("Board saved with tag: {tag}")),
        }
        .process(server.self_endpoint, server)
        .await?;

        info!("sent log message");

        Ok(())
    }
}

impl ServerTask for LoadBoard {
    async fn process(self, endpoint: Endpoint, server: &DndServer) -> anyhow::Result<()> {
        let resp = server
            .db
            .from("board_data")
            .eq("tag", &self.tag)
            .select("data")
            .order_with_options("created_at", None::<String>, false, false)
            .execute()
            .await?;

        #[derive(serde::Deserialize)]
        struct ServerData {
            data: common::board::BoardData,
        }

        let data = resp.text_with_error().await?;

        info!("Latest board data {data}");

        let all_saves: Vec<ServerData> = serde_json::from_str(&data)?;
        let board_data = all_saves.into_iter().next().map(|x| x.data);

        match board_data {
            Some(board_data) => {
                server.board_data.overwrite_board_data(board_data);
                Log {
                    user: common::User::server(),
                    payload: LogMessage::Server(format!("Loaded board: {}", self.tag)),
                }
                .process(server.self_endpoint, server)
                .await?;

                // TODO: It would be better if the client and server had same representation of board so
                // that I could send the full state and not individual commands for each piece
                //BroadcastBoardMessage(BoardMessage::ClearBoard)
                //    .process(server.self_endpoint, server)
                //    .await?;

                BroadcastAllBoardData.process(endpoint, server).await?;
            }
            None => {
                DirectMessage(Log {
                    user: common::User::server(),
                    payload: LogMessage::Server(format!(
                        "Requested board save does not exist: {}",
                        self.tag
                    )),
                })
                .process(endpoint, server)
                .await?;
            }
        }

        Ok(())
    }
}

pub struct BroadcastAllBoardData;
impl ServerTask for BroadcastAllBoardData {
    async fn process(self, _: Endpoint, server: &DndServer) -> anyhow::Result<()> {
        // TODO: It would be better if the client and server had same representation of board so
        // that I could send the full state and not individual commands for each piece
        //for (uuid, player) in server.board_data.get_players_owned().into_iter() {
        //    let message = DndMessage::BoardMessage(BoardMessage::AddPlayerPiece(uuid, player));

        //    let output_data = bincode::serialize(&message)?;

        //    server.users.foreach(|(_name, user)| {
        //        server
        //            .handler
        //            .network()
        //            .send(user.endpoint, &output_data)
        //            .to_error()
        //    })?;
        //}

        Ok(())
    }
}

pub struct SendInitialBoardData;
impl ServerTask for SendInitialBoardData {
    async fn process(self, endpoint: Endpoint, server: &DndServer) -> anyhow::Result<()> {
        //for (uuid, player) in server.board_data.get_players_owned().into_iter() {
        //    let message = DndMessage::BoardMessage(BoardMessage::AddPlayerPiece(uuid, player));

        //    let output_data = bincode::serialize(&message)?;

        //    server
        //        .handler
        //        .network()
        //        .send(endpoint, &output_data)
        //        .to_error()?;
        //}

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
            data: common::board::BoardData,
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

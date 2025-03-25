use common::{
    message::{DndMessage, Log, LogMessage},
    User,
};
use log::debug;
use message_io::network::Endpoint;

use crate::DndServer;

use super::{Broadcast, Response};

impl Response for Log {
    type Action = Broadcast;
    type ResponseData = DndMessage;

    async fn response(self, _: Endpoint, _: &DndServer) -> anyhow::Result<Self::ResponseData> {
        debug!("Broadcasting log message!");
        Ok(DndMessage::Log(self))
    }
}

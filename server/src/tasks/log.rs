use common::{
    message::{DndMessage, Log, LogMessage},
    User,
};
use log::debug;
use message_io::network::Endpoint;

use crate::DndServer;

use super::{Broadcast, Response, ReturnToSender};

impl Response for Log {
    type Action = Broadcast;
    type ResponseData = DndMessage;

    async fn response(self, _: Endpoint, _: &DndServer) -> anyhow::Result<Self::ResponseData> {
        debug!("Broadcasting log message!");
        Ok(DndMessage::Log(self))
    }
}

pub struct DirectMessage(pub Log);
impl Response for DirectMessage {
    type Action = ReturnToSender;
    type ResponseData = DndMessage;

    async fn response(
        self,
        endpoint: Endpoint,
        _: &DndServer,
    ) -> anyhow::Result<Self::ResponseData> {
        debug!("Sending direct message to {endpoint:?}");

        Ok(DndMessage::Log(self.0))
    }
}

use common::{
    message::{DndMessage, LogMessage},
    User,
};
use log::debug;

use super::{Broadcast, Response};

pub struct BroadcastLogMsg {
    sender: User,
    msg: LogMessage,
}

impl BroadcastLogMsg {
    pub fn new(sender: User, msg: LogMessage) -> Self {
        Self { sender, msg }
    }
}

impl Response for BroadcastLogMsg {
    type Action = Broadcast;
    type ResponseData = DndMessage;

    async fn response(
        self,
        _: message_io::network::Endpoint,
        _: &crate::DndServer,
    ) -> anyhow::Result<Self::ResponseData> {
        let Self { sender, msg } = self;

        debug!("Broadcasting log message!");
        Ok(DndMessage::Log(sender, msg))
    }
}

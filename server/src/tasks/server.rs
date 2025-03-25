use common::{
    message::{DndMessage, Log, LogMessage, RegisterUser, RetrieveCharacterData, UnRegisterUser},
    User,
};
use log::info;
use message_io::network::Endpoint;

use crate::{tasks::board::*, tasks::db::*, ClientInfo, ServerError};

use super::{Broadcast, Response, ReturnToSender, ServerTask};

impl ServerTask for RegisterUser {
    async fn process(
        self,
        endpoint: Endpoint,
        server: &mut crate::DndServer,
    ) -> anyhow::Result<()> {
        let Self { name } = self;
        if server.users.contains_key(&name) {
            return Err(ServerError::UserAlreadyExists(name).into());
        }

        UserList.process(endpoint, server).await?;
        GetCharacterList.process(endpoint, server).await?;
        UserAdded(name.clone()).process(endpoint, server).await?;

        server.users.insert(
            name.clone(),
            ClientInfo {
                user_data: User {
                    name: name.to_string(),
                },
                endpoint,
            },
        );

        info!("Added user '{}'", name);

        Log {
            user: User::server(),
            payload: LogMessage::Joined(name),
        }
        .process(endpoint, server)
        .await?;

        Ok(())
    }
}

impl ServerTask for UnRegisterUser {
    async fn process(
        self,
        endpoint: Endpoint,
        server: &mut crate::DndServer,
    ) -> anyhow::Result<()> {
        let Self { name } = self;

        let info = server
            .users
            .remove(&name)
            .ok_or_else(|| ServerError::UserNotFound(name.clone()))?;

        UserRemoved(info.user_data.name)
            .process(endpoint, server)
            .await?;

        Log {
            user: User::server(),
            payload: LogMessage::Disconnected(name.clone()),
        }
        .process(endpoint, server)
        .await?;

        info!("Removed participant '{}'", name);

        Ok(())
    }
}

impl ServerTask for RetrieveCharacterData {
    async fn process(
        self,
        endpoint: Endpoint,
        server: &mut crate::DndServer,
    ) -> anyhow::Result<()> {
        let Self { user } = self;
        GetItemList(&user).process(endpoint, server).await?;
        GetAbilityList(&user).process(endpoint, server).await?;
        GetCharacterStats(&user).process(endpoint, server).await?;
        SendInitialBoardData.process(endpoint, server).await?;

        Ok(())
    }
}

pub struct UserList;
impl Response for UserList {
    type Action = ReturnToSender;
    type ResponseData = DndMessage;

    async fn response(
        self,
        _: Endpoint,
        server: &crate::DndServer,
    ) -> anyhow::Result<Self::ResponseData> {
        let list = server.users.keys().cloned().collect();
        Ok(DndMessage::UserList(list))
    }
}

/// Notify other users about this new user
pub struct UserAdded(pub String);
impl Response for UserAdded {
    type Action = Broadcast;
    type ResponseData = DndMessage;

    async fn response(
        self,
        _: Endpoint,
        _: &crate::DndServer,
    ) -> anyhow::Result<Self::ResponseData> {
        Ok(DndMessage::UserNotificationAdded(self.0))
    }
}

/// Notify other users about this new user
pub struct UserRemoved(pub String);
impl Response for UserRemoved {
    type Action = Broadcast;
    type ResponseData = DndMessage;

    async fn response(
        self,
        _: Endpoint,
        _: &crate::DndServer,
    ) -> anyhow::Result<Self::ResponseData> {
        Ok(DndMessage::UserNotificationRemoved(self.0))
    }
}

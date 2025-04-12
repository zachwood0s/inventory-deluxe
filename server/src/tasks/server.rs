use common::{
    message::{DndMessage, Log, LogMessage, RegisterUser, RetrieveCharacterData, UnRegisterUser},
    User,
};
use log::info;

use crate::{
    tasks::{board::*, data_store::SendLatestDbData, db::*},
    ClientInfo, DndEndpoint, DndServer, ListenerCtx, ServerError,
};

use super::{Broadcast, Response, ReturnToSender, ServerTask};

impl ServerTask for RegisterUser {
    async fn process(
        self,
        endpoint: DndEndpoint,
        server: &DndServer,
        ctx: &ListenerCtx,
    ) -> anyhow::Result<()> {
        let Self { name } = self;
        if server.users.has_user(&name) {
            return Err(ServerError::UserAlreadyExists(name).into());
        }

        tokio::try_join!(
            UserList.process(endpoint, server, ctx),
            SendLatestDbData.process(endpoint, server, ctx),
            UserAdded(name.clone()).process(endpoint, server, ctx),
            Log {
                user: User::server(),
                payload: LogMessage::Joined(name.clone()),
            }
            .process(endpoint, server, ctx),
            InsertUser(name.clone()).process(endpoint, server, ctx)
        )?;

        info!("Added user '{}'", name);

        Ok(())
    }
}

pub struct InsertUser(String);
impl ServerTask for InsertUser {
    async fn process(
        self,
        endpoint: DndEndpoint,
        server: &DndServer,
        _: &ListenerCtx,
    ) -> anyhow::Result<()> {
        let Self(name) = self;

        server.users.insert_user(
            name.clone(),
            ClientInfo {
                user_data: User { name },
                endpoint,
            },
        );

        Ok(())
    }
}

impl ServerTask for UnRegisterUser {
    async fn process(
        self,
        endpoint: DndEndpoint,
        server: &DndServer,
        ctx: &ListenerCtx,
    ) -> anyhow::Result<()> {
        let Self { name } = self;

        let info = server
            .users
            .remove_user(&name)
            .ok_or_else(|| ServerError::UserNotFound(name.clone()))?;

        UserRemoved(info.user_data.name)
            .process(endpoint, server, ctx)
            .await?;

        Log {
            user: User::server(),
            payload: LogMessage::Disconnected(name.clone()),
        }
        .process(endpoint, server, ctx)
        .await?;

        info!("Removed participant '{}'", name);

        Ok(())
    }
}

impl ServerTask for RetrieveCharacterData {
    async fn process(
        self,
        endpoint: DndEndpoint,
        server: &DndServer,
        ctx: &ListenerCtx,
    ) -> anyhow::Result<()> {
        let Self { user } = self;

        tokio::try_join!(
            GetItemList(&user).process(endpoint, server, ctx),
            GetAbilityList(&user).process(endpoint, server, ctx),
            GetCharacterStats(&user).process(endpoint, server, ctx),
            SendInitialBoardData.process(endpoint, server, ctx),
        )?;

        Ok(())
    }
}

pub struct UserList;
impl Response for UserList {
    type Action = ReturnToSender;
    type ResponseData = DndMessage;

    async fn response(
        self,
        _: DndEndpoint,
        server: &DndServer,
    ) -> anyhow::Result<Self::ResponseData> {
        let list = server.users.users_names_owned();
        Ok(DndMessage::UserList(list))
    }
}

/// Notify other users about this new user
pub struct UserAdded(pub String);
impl Response for UserAdded {
    type Action = Broadcast;
    type ResponseData = DndMessage;

    async fn response(self, _: DndEndpoint, _: &DndServer) -> anyhow::Result<Self::ResponseData> {
        Ok(DndMessage::UserNotificationAdded(self.0))
    }
}

/// Notify other users about this new user
pub struct UserRemoved(pub String);
impl Response for UserRemoved {
    type Action = Broadcast;
    type ResponseData = DndMessage;

    async fn response(self, _: DndEndpoint, _: &DndServer) -> anyhow::Result<Self::ResponseData> {
        Ok(DndMessage::UserNotificationRemoved(self.0))
    }
}

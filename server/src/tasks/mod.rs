use crate::{DndEndpoint, DndServer, ListenerCtx, ToError};

pub mod board;
pub mod db;
pub mod log;
pub mod server;

pub trait ServerTask {
    fn process(
        self,
        endpoint: DndEndpoint,
        server: &DndServer,
        ctx: &ListenerCtx,
    ) -> impl futures::Future<Output = anyhow::Result<()>> + Send;
}

pub struct ReturnToSender;
pub struct Broadcast;

pub trait Response: Send + Sync {
    type Action;
    // TODO: Should I just restrict this to Into<DNDMessage>?
    type ResponseData: serde::Serialize;
    fn response(
        self,
        endpoint: DndEndpoint,
        server: &DndServer,
    ) -> impl futures::Future<Output = anyhow::Result<Self::ResponseData>> + Send;
}

pub trait ResponseAction<T: Response> {
    fn do_action(
        t: T,
        endpoint: DndEndpoint,
        server: &DndServer,
        ctx: &ListenerCtx,
    ) -> impl futures::Future<Output = anyhow::Result<()>> + Send;
}

impl<T> ServerTask for T
where
    T: Response,
    <T as Response>::Action: ResponseAction<T>,
{
    async fn process(
        self,
        endpoint: DndEndpoint,
        server: &DndServer,
        ctx: &ListenerCtx,
    ) -> anyhow::Result<()> {
        T::Action::do_action(self, endpoint, server, ctx).await
    }
}

impl<T> ResponseAction<T> for ReturnToSender
where
    T: Response,
{
    async fn do_action(
        t: T,
        endpoint: DndEndpoint,
        server: &DndServer,
        ctx: &ListenerCtx,
    ) -> anyhow::Result<()> {
        let resp_msg = t.response(endpoint, server).await?;
        let encoded = bincode::serialize(&resp_msg)?;

        // Only support sending responses to clients
        let endpoint = endpoint.client()?;

        ctx.handler.network().send(endpoint, &encoded).to_error()?;

        Ok(())
    }
}

impl<T> ResponseAction<T> for Broadcast
where
    T: Response,
{
    async fn do_action(
        t: T,
        endpoint: DndEndpoint,
        server: &DndServer,
        ctx: &ListenerCtx,
    ) -> anyhow::Result<()> {
        let resp_msg = t.response(endpoint, server).await?;
        let encoded = bincode::serialize(&resp_msg)?;

        server.users.foreach(|(_name, user)| {
            if user.endpoint != endpoint {
                // All users should be client endpoints
                let user_endpoint = user.endpoint.client()?;
                ctx.handler
                    .network()
                    .send(user_endpoint, &encoded)
                    .to_error()?;
            }

            Ok(())
        })
    }
}

pub trait DbTask {
    fn builder(
        self,
        db: &postgrest::Postgrest,
    ) -> impl futures::Future<Output = anyhow::Result<postgrest::Builder>> + Send;
}

//impl<T> ServerTask for T
//where
//    T: DbTask,
//{
//    async fn process(
//        self,
//        endpoint: DndEndpoint,
//        server: &DndServer,
//        ctx: &ListenerCtx,
//    ) -> anyhow::Result<()> {
//        todo!()
//    }
//}

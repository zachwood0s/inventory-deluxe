use futures::{future::try_join, try_join, FutureExt};
use message_io::network::Endpoint;

use crate::{DndServer, ToError};

pub mod board;
pub mod db;
pub mod log;
pub mod server;

pub trait ServerTask {
    fn process(
        self,
        endpoint: Endpoint,
        server: &DndServer,
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
        endpoint: Endpoint,
        server: &DndServer,
    ) -> impl futures::Future<Output = anyhow::Result<Self::ResponseData>> + Send;
}

pub trait ResponseAction<T: Response> {
    fn do_action(
        t: T,
        endpoint: Endpoint,
        server: &DndServer,
    ) -> impl futures::Future<Output = anyhow::Result<()>> + Send;
}

impl<T> ServerTask for T
where
    T: Response,
    <T as Response>::Action: ResponseAction<T>,
{
    async fn process(self, endpoint: Endpoint, server: &DndServer) -> anyhow::Result<()> {
        T::Action::do_action(self, endpoint, server).await
    }
}

impl<T> ResponseAction<T> for ReturnToSender
where
    T: Response,
{
    async fn do_action(t: T, endpoint: Endpoint, server: &DndServer) -> anyhow::Result<()> {
        let resp_msg = t.response(endpoint, server).await?;
        let encoded = bincode::serialize(&resp_msg)?;
        server
            .handler
            .network()
            .send(endpoint, &encoded)
            .to_error()?;

        Ok(())
    }
}

impl<T> ResponseAction<T> for Broadcast
where
    T: Response,
{
    async fn do_action(t: T, endpoint: Endpoint, server: &DndServer) -> anyhow::Result<()> {
        let resp_msg = t.response(endpoint, server).await?;
        let encoded = bincode::serialize(&resp_msg)?;

        server.users.foreach(|(_name, user)| {
            if user.endpoint != endpoint {
                server
                    .handler
                    .network()
                    .send(user.endpoint, &encoded)
                    .to_error()?;
            }

            Ok(())
        })
    }
}

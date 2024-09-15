use std::{
    collections::HashMap,
    io,
    sync::mpsc::{Receiver, Sender},
};

use common::message::DndMessage;
use message_io::{
    events::EventSender,
    network::{Endpoint, NetEvent, Transport},
    node::{self, NodeHandler, NodeListener},
};

pub enum Signal {
    ClientMessage(DndMessage),
}

impl From<DndMessage> for Signal {
    fn from(value: DndMessage) -> Self {
        Signal::ClientMessage(value)
    }
}

pub struct DndListener {
    handler: NodeHandler<Signal>,
    node_listener: Option<NodeListener<Signal>>,
    server_endpoint: Endpoint,
    tx: Sender<DndMessage>,
}

impl DndListener {
    pub fn new(tx: Sender<DndMessage>) -> io::Result<Self> {
        let (handler, node_listener) = node::split();
        let server_addr = "127.0.0.1:80";
        let (endpoint, _) = handler.network().connect(Transport::Ws, server_addr)?;

        Ok(Self {
            handler,
            node_listener: Some(node_listener),
            server_endpoint: endpoint,
            tx,
        })
    }

    pub fn event_sender(&self) -> EventSender<Signal> {
        self.handler.signals().clone()
    }

    pub fn run(mut self) {
        let node_listener = self.node_listener.take().unwrap();

        node_listener.for_each(move |event| match event {
            node::NodeEvent::Network(net_event) => match net_event {
                NetEvent::Connected(endpoint, established) => {
                    if endpoint == self.server_endpoint {
                        if established {
                            let message = DndMessage::RegisterUser("JoingleBob".into());
                            let output_data = bincode::serialize(&message).unwrap();
                            self.handler
                                .network()
                                .send(self.server_endpoint, &output_data);
                        } else {
                            println!("Could not connect to the server");
                        }
                    }
                }
                NetEvent::Accepted(_, _) => (),
                NetEvent::Message(_, input_data) => {
                    let message: DndMessage = bincode::deserialize(input_data).unwrap();

                    println!("Recieved message from server {message:?}");

                    self.tx.send(message).unwrap();
                }
                NetEvent::Disconnected(_) => {
                    println!("Server is disconnected");
                    self.handler.stop();
                }
            },
            node::NodeEvent::Signal(signal) => match signal {
                Signal::ClientMessage(msg) => {
                    let input_data = bincode::serialize(&msg).unwrap();
                    self.handler
                        .network()
                        .send(self.server_endpoint, &input_data);
                }
            },
        })
    }
}

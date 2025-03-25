use std::{
    collections::HashMap,
    io,
    sync::mpsc::{Receiver, Sender},
};

use common::{
    message::{DndMessage, RegisterUser, RetrieveCharacterData},
    User,
};
use message_io::{
    events::EventSender,
    network::{Endpoint, NetEvent, Transport},
    node::{self, NodeHandler, NodeListener},
};

use crate::state::DndState;

pub enum Signal {
    ClientMessage(DndMessage),
    RecieveMessage(DndMessage),
}

impl From<DndMessage> for Signal {
    fn from(value: DndMessage) -> Self {
        Signal::ClientMessage(value)
    }
}

pub trait Command {
    fn execute(self: Box<Self>, state: &mut DndState, tx: &EventSender<Signal>);
}

pub struct CommandQueue<'a> {
    pub command_queue: &'a mut Vec<Box<dyn Command>>,
}

impl<'a> CommandQueue<'a> {
    pub fn add<T: Command + 'static>(&mut self, command: T) {
        self.command_queue.push(Box::new(command));
    }
}

pub struct DndListener {
    user: User,
    handler: NodeHandler<Signal>,
    node_listener: Option<NodeListener<Signal>>,
    server_endpoint: Endpoint,
    tx: Sender<DndMessage>,
}

impl DndListener {
    pub fn new(tx: Sender<DndMessage>, user: User, server_addr: &str) -> io::Result<Self> {
        let (handler, node_listener) = node::split();
        let (endpoint, _) = handler.network().connect(Transport::Ws, server_addr)?;

        Ok(Self {
            user,
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
                            let message = DndMessage::RegisterUser(RegisterUser {
                                name: self.user.name.clone(),
                            });
                            let output_data = bincode::serialize(&message).unwrap();
                            self.handler
                                .network()
                                .send(self.server_endpoint, &output_data);

                            let message =
                                DndMessage::RetrieveCharacterData(RetrieveCharacterData {
                                    user: self.user.clone(),
                                });
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

                    self.handler.signals().send(Signal::RecieveMessage(message));
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

                    // Immediately send the message back to ourself
                    //if matches!(msg, DndMessage::BoardMessage(_)) {
                    self.handler.signals().send(Signal::RecieveMessage(msg))
                    //}
                }
                Signal::RecieveMessage(msg) => {
                    self.tx.send(msg).unwrap();
                }
            },
        })
    }
}

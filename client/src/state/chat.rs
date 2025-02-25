use crate::prelude::*;
use egui::{text::LayoutJob, Align, Color32, FontSelection, RichText, Style};

pub struct ClientLogMessage {
    pub user: User,
    pub message: LogMessage,
}

impl ClientLogMessage {
    pub fn new(user: User, message: LogMessage) -> Self {
        Self { user, message }
    }

    pub fn ui(&self, ui: &mut egui::Ui, display_name: bool) {
        let hide_name = matches!(self.message, LogMessage::Joined(_))
            || matches!(self.message, LogMessage::Disconnected(_));

        if display_name {
            ui.separator();
            if !hide_name {
                ui.colored_label(Color32::LIGHT_BLUE, format!("{}: ", self.user.name));
            }
        }

        match &self.message {
            LogMessage::Chat(c) => {
                ui.label(c);
            }
            LogMessage::UseItem(item, count) => {
                let style = Style::default();
                let mut layout_job = LayoutJob::default();
                RichText::new(format!("Used {} ", count))
                    .italics()
                    .append_to(&mut layout_job, &style, FontSelection::Default, Align::LEFT);

                RichText::new(item).color(Color32::LIGHT_GREEN).append_to(
                    &mut layout_job,
                    &style,
                    FontSelection::Default,
                    Align::LEFT,
                );

                ui.label(layout_job);
            }
            LogMessage::Joined(joined_user) => {
                ui.colored_label(Color32::DARK_GRAY, format!("{} joined", joined_user));
            }
            LogMessage::Disconnected(discon_user) => {
                ui.colored_label(Color32::DARK_GRAY, format!("{} disconnected", discon_user));
            }
            LogMessage::SetAbilityCount(ability, count) => {
                let style = Style::default();
                let mut layout_job = LayoutJob::default();
                RichText::new("Used  ").italics().append_to(
                    &mut layout_job,
                    &style,
                    FontSelection::Default,
                    Align::LEFT,
                );

                RichText::new(ability).color(Color32::LIGHT_RED).append_to(
                    &mut layout_job,
                    &style,
                    FontSelection::Default,
                    Align::LEFT,
                );

                RichText::new(format!(", they have {} uses left", count))
                    .italics()
                    .append_to(&mut layout_job, &style, FontSelection::Default, Align::LEFT);

                ui.label(layout_job);
            }
            LogMessage::Roll(die, value) => {
                ui.colored_label(Color32::DARK_GRAY, format!("d{} = {}", die, value));
            }
        };
    }
}

#[derive(Default)]
pub struct ChatState {
    pub log_messages: Vec<ClientLogMessage>,
}

impl ChatState {
    pub fn process(&mut self, message: &DndMessage) {
        #[allow(clippy::single_match)]
        match message {
            DndMessage::Log(user, msg) => self
                .log_messages
                .push(ClientLogMessage::new(user.clone(), msg.clone())),
            DndMessage::ItemList(list) => {
                println!("Recieved item list {list:?}");
            }
            _ => {}
        }
    }
}

pub mod commands {

    use itertools::Itertools;
    use rand::Rng;
    use thiserror::Error;

    use crate::prelude::*;

    pub struct ChatCommand {
        text: String,
    }

    impl ChatCommand {
        pub fn new(text: String) -> Self {
            Self { text }
        }

        fn parse_cmd(
            &self,
            cmd: &str,
            state: &mut DndState,
        ) -> Result<DndMessage, ChatCommandError> {
            let cmd_parts = cmd.split(" ").collect_vec();
            match cmd_parts.first() {
                // roll
                Some(&"roll") | Some(&"r") | Some(&"d") => {
                    let roll = *cmd_parts
                        .get(1)
                        .ok_or(ChatCommandError::ExpectedMoreArgs(1))?;

                    roll_die(roll)
                        .map(|(die, val)| {
                            DndMessage::Log(state.owned_user(), LogMessage::Roll(die, val))
                        })
                        .map_err(|e| e.into())
                }
                // add more cmds if you want cale
                _ => Err(ChatCommandError::BadCommand),
            }
        }
    }

    impl Command for ChatCommand {
        fn execute(self: Box<Self>, state: &mut DndState, tx: &EventSender<Signal>) {
            let mut text_it = self.text.chars();
            match text_it.next() {
                Some('/') => {
                    let cmd = text_it.as_str();
                    match self.parse_cmd(cmd, state) {
                        Ok(msg) => tx.send(msg.into()),
                        Err(e) => {
                            error!("Error parsing command: {e}")
                        }
                    };
                }
                // little special case here for d for dnd
                Some('d') => {
                    let die = ["d ", text_it.as_str()].concat();
                    match self.parse_cmd(&die, state) {
                        Ok(msg) => tx.send(msg.into()),
                        _ => tx.send(
                            DndMessage::Log(state.owned_user(), LogMessage::Chat(self.text)).into(),
                        ),
                    };
                }
                None => {}
                _ => {
                    tx.send(DndMessage::Log(state.owned_user(), LogMessage::Chat(self.text)).into())
                }
            }
        }
    }

    #[derive(Error, Debug)]
    enum ChatCommandError {
        #[error("bad cmd try again")]
        BadCommand,
        #[error("stupid stupid stupdi; needed {0} args")]
        ExpectedMoreArgs(u32),
        #[error("error parsing dice roll {0}")]
        DiceRollError(#[from] DiceRollError),
    }

    #[derive(Error, Debug)]
    enum DiceRollError {
        #[error("Failed to parse the dice number")]
        ParseError(#[from] std::num::ParseIntError),
    }

    fn roll_die(roll: &str) -> Result<(u32, u32), DiceRollError> {
        let die = roll.parse()?;
        let mut rng = rand::rng();
        let die_val: u32 = rng.random_range(0..die);
        let die_tuple = (die, die_val);

        Ok(die_tuple)
    }
}

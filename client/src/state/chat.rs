use crate::prelude::*;
use comand_parser::{ChatCommandParser, ToDndMessge};
use egui::{text::LayoutJob, Align, Color32, FontSelection, RichText, Style};
use itertools::Itertools;

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
            LogMessage::Server(msg) => {
                ui.colored_label(Color32::DARK_GRAY, msg);
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
            DndMessage::Log(Log { user, payload }) => self
                .log_messages
                .push(ClientLogMessage::new(user.clone(), payload.clone())),
            DndMessage::ItemList(list) => {
                println!("Recieved item list {list:?}");
            }
            _ => {}
        }
    }
}

pub struct ChatCommand {
    text: String,
}

impl ChatCommand {
    pub fn new(text: String) -> Self {
        Self { text }
    }
}

impl Command for ChatCommand {
    fn execute(self: Box<Self>, state: &mut DndState, tx: &EventSender<Signal>) {
        let parts = self.text.split(" ").collect_vec();

        type Parser = (commands::Roll, commands::SaveBoard, commands::LoadBoard);
        match Parser::parse(&parts) {
            Some(command) => tx.send(command.to_dnd(state).into()),
            None => {
                tx.send(
                    DndMessage::Log(Log {
                        user: state.owned_user(),
                        payload: LogMessage::Chat(self.text),
                    })
                    .into(),
                );
            }
        }
    }
}

mod comand_parser {
    use common::message::DndMessage;
    use egui::TextBuffer;

    use super::DndState;

    pub trait ToDndMessge {
        fn to_dnd(&self, state: &DndState) -> DndMessage;
    }

    impl ToDndMessge for DndMessage {
        fn to_dnd(&self, _state: &DndState) -> DndMessage {
            self.clone()
        }
    }

    impl<T: ToDndMessge + ?Sized> ToDndMessge for Box<T> {
        fn to_dnd(&self, state: &DndState) -> DndMessage {
            (**self).to_dnd(state)
        }
    }

    pub trait ChatCommandParser {
        type Output: ToDndMessge + 'static;

        fn parse(parts: &[&str]) -> Option<Self::Output> {
            let first = parts.first()?;

            Self::prefix()
                .contains(&first.as_str())
                .then(|| Self::parse_parts(parts))
                .flatten()
        }

        fn prefix() -> Vec<&'static str>;
        fn parse_parts(parts: &[&str]) -> Option<Self::Output>;
    }

    macro_rules! impl_command_tuple {
        (
            $($type: ident),*
        ) => {
            impl<$($type: ChatCommandParser, )*> ChatCommandParser for ($($type, )*) {
                type Output = Box<dyn ToDndMessge>;

                fn prefix() -> Vec<&'static str> {
                    [
                        $($type::prefix(),)*
                    ].into_iter().flatten().collect()
                }

                fn parse_parts(parts: &[&str]) -> Option<Self::Output> {
                    $(
                        if let Some(res) = $type::parse(parts) {
                            return Some(Box::new(res));
                        }
                    )*

                    None
                }
            }
        }
    }

    impl_command_tuple!(T1, T2);
    impl_command_tuple!(T1, T2, T3);
    impl_command_tuple!(T1, T2, T3, T4);
    impl_command_tuple!(T1, T2, T3, T4, T5);
    impl_command_tuple!(T1, T2, T3, T4, T5, T6);
    impl_command_tuple!(T1, T2, T3, T4, T5, T6, T7);
}

pub mod commands {
    use common::message::{DndMessage, Log, LogMessage};
    use once_cell::sync::Lazy;
    use rand::Rng;
    use regex::Regex;

    use super::{
        comand_parser::{ChatCommandParser, ToDndMessge},
        DndState,
    };

    pub struct Roll {
        die: u32,
        val: u32,
    }

    pub enum Adv {
        Advantage,
        Disadvantage,
        None,
    }

    impl From<&str> for Adv {
        fn from(value: &str) -> Self {
            match value {
                "+" => Adv::Advantage,
                "-" => Adv::Disadvantage,
                _ => Adv::None,
            }
        }
    }

    impl ToDndMessge for Roll {
        fn to_dnd(&self, state: &DndState) -> DndMessage {
            DndMessage::Log(Log {
                user: state.owned_user(),
                payload: LogMessage::Roll(self.die, self.val),
            })
        }
    }

    impl ChatCommandParser for Roll {
        type Output = Self;

        fn prefix() -> Vec<&'static str> {
            vec!["/roll", "/r"]
        }

        fn parse_parts(parts: &[&str]) -> Option<Self::Output> {
            static RE: Lazy<Regex> = Lazy::new(|| {
                Regex::new(r"(?P<amount>[0-9]*)d(?P<die>[0-9]+)(?P<adv>[\+-])?").unwrap()
            });

            if parts.len() != 2 {
                return None;
            }

            let captures = RE.captures(parts[1])?;

            let get_capture_int = |name, default| -> Option<u32> {
                captures
                    .name(name)
                    .map_or(Some(default), |x| x.as_str().parse().ok())
            };

            let die_count = get_capture_int("amount", 1)?;
            let die = get_capture_int("die", 1)?;
            let adv = captures
                .name("adv")
                .map_or(Adv::None, |x| x.as_str().into());

            let mut rng = rand::rng();

            Some(Roll {
                die,
                val: rng.random_range(1..=die),
            })
        }
    }

    pub struct SaveBoard {
        tag: Option<String>,
    }

    impl ToDndMessge for SaveBoard {
        fn to_dnd(&self, _: &DndState) -> DndMessage {
            DndMessage::SaveBoard(common::message::SaveBoard {
                tag: self.tag.clone(),
            })
        }
    }

    impl ChatCommandParser for SaveBoard {
        type Output = Self;

        fn prefix() -> Vec<&'static str> {
            vec!["/save"]
        }

        fn parse_parts(parts: &[&str]) -> Option<Self::Output> {
            if parts.len() > 2 {
                return None;
            }

            let tag = parts.get(1).map(|x| String::from(*x));

            Some(Self { tag })
        }
    }

    pub struct LoadBoard {
        tag: String,
    }

    impl ToDndMessge for LoadBoard {
        fn to_dnd(&self, _: &DndState) -> DndMessage {
            DndMessage::LoadBoard(common::message::LoadBoard {
                tag: self.tag.clone(),
            })
        }
    }

    impl ChatCommandParser for LoadBoard {
        type Output = Self;

        fn prefix() -> Vec<&'static str> {
            vec!["/load"]
        }

        fn parse_parts(parts: &[&str]) -> Option<Self::Output> {
            if parts.len() != 2 {
                return None;
            }

            let tag = parts.get(1).map(|x| String::from(*x))?;

            Some(Self { tag })
        }
    }

    impl ToDndMessge for String {
        fn to_dnd(&self, state: &DndState) -> DndMessage {
            DndMessage::Log(Log {
                user: state.owned_user(),
                payload: LogMessage::Chat(self.clone()),
            })
        }
    }
}

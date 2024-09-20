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

                RichText::new(format!("{}", item))
                    .color(Color32::LIGHT_GREEN)
                    .append_to(&mut layout_job, &style, FontSelection::Default, Align::LEFT);

                ui.label(layout_job);
            }
            LogMessage::Joined(joined_user) => {
                ui.colored_label(Color32::DARK_GRAY, format!("{} joined", joined_user));
            }
            LogMessage::Disconnected(discon_user) => {
                ui.colored_label(Color32::DARK_GRAY, format!("{} disconnected", discon_user));
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
    use crate::prelude::*;

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
            tx.send(DndMessage::Log(state.owned_user(), LogMessage::Chat(self.text)).into())
        }
    }
}

use common::{message::DndMessage, User};
use egui::Color32;

pub struct LogMessage {
    pub user: User,
    pub message: String,
}

impl LogMessage {
    pub fn new(user: User, message: String) -> Self {
        Self { user, message }
    }

    pub fn ui(&self, ui: &mut egui::Ui, display_name: bool) {
        if display_name {
            ui.separator();
            ui.colored_label(Color32::LIGHT_BLUE, format!("{}: ", self.user.name));
        }
        ui.label(&self.message);
    }
}

#[derive(Default)]
pub struct ChatState {
    pub log_messages: Vec<LogMessage>,
}

impl ChatState {
    pub fn process(&mut self, message: &DndMessage) {
        #[allow(clippy::single_match)]
        match message {
            DndMessage::Chat(user, msg) => self
                .log_messages
                .push(LogMessage::new(user.clone(), msg.clone())),
            DndMessage::ItemList(list) => {
                println!("Recieved item list {list:?}");
            }
            _ => {}
        }
    }
}

use common::{message::DndMessage, User};
use egui::Color32;

pub struct LogMessage {
    user: User,
    message: String,
}

impl LogMessage {
    pub fn new(user: User, message: String) -> Self {
        Self { user, message }
    }

    pub fn ui(&self, ui: &mut egui::Ui) {
        ui.colored_label(Color32::LIGHT_BLUE, format!("{}: ", self.user.name));
        ui.label(&self.message);
        ui.separator();
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
            _ => {}
        }
    }
}

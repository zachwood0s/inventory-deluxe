use std::sync::mpsc::Receiver;

use common::{message::DndMessage, User};
use egui::{Color32, ScrollArea, TextEdit, Widget};
use message_io::events::EventSender;

use crate::listener::Signal;

use super::DndTabImpl;

struct LogMessage {
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
pub struct Chat {
    log_messages: Vec<LogMessage>,
    text: String,
}

impl DndTabImpl for Chat {
    fn ui(&mut self, ui: &mut egui::Ui, tx: &EventSender<Signal>, rx: &Receiver<DndMessage>) {
        egui::TopBottomPanel::bottom("chat_box")
            .resizable(false)
            .min_height(30.0)
            .show_inside(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    let submitted = TextEdit::singleline(&mut self.text)
                        .desired_width(f32::INFINITY)
                        .ui(ui);

                    if submitted.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let user = common::User {
                            name: "JoingleBob".into(),
                        };

                        tx.send(DndMessage::Chat(user, self.text.clone()).into())
                    }
                })
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ScrollArea::new([false, true]).show(ui, |ui| {
                for msg in self.log_messages.iter() {
                    msg.ui(ui);
                }
            });
        });

        for msg in rx.try_iter() {
            #[allow(clippy::single_match)]
            match msg {
                DndMessage::Chat(user, msg) => self.log_messages.push(LogMessage::new(user, msg)),
                _ => {}
            }
        }
    }

    fn title(&self) -> String {
        "Chat".to_owned()
    }
}

use common::Item;
use egui::{
    collapsing_header, panel::TopBottomSide, popup_below_widget, text::LayoutJob, tooltip_id,
    CentralPanel, CollapsingHeader, Color32, DragValue, Frame, Label, Margin, Resize, RichText,
    TopBottomPanel, Widget,
};
use egui_extras::{Strip, StripBuilder};

use super::DndTabImpl;

pub struct StatWidget {
    name: String,
    value: u32,
}

impl StatWidget {
    pub fn new(name: impl ToString, value: u32) -> Self {
        Self {
            name: name.to_string(),
            value,
        }
    }
}

impl egui::Widget for StatWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        Frame::none()
            .stroke(egui::Stroke {
                width: 1.0,
                color: Color32::LIGHT_GRAY,
            })
            .inner_margin(Margin::same(5.0))
            .show(ui, |ui| {
                Resize::default()
                    .resizable(false)
                    .default_size([40.0, 40.0])
                    .show(ui, |ui| {
                        ui.vertical_centered_justified(|ui| {
                            ui.label(self.name);
                            ui.heading(self.value.to_string());
                        });
                    });
            })
            .response
    }
}

#[derive(Default)]
pub struct Character {
    use_num: u32,
}

impl Character {
    fn draw_item(&mut self, ui: &mut egui::Ui, item: &Item) {
        let mut item_text =
            LayoutJob::single_section(item.name.clone(), egui::TextFormat::default());
        item_text.append(
            &format!("x{}", item.count),
            10.0,
            egui::TextFormat {
                color: Color32::LIGHT_GREEN,
                italics: true,
                ..Default::default()
            },
        );
        let id = egui::Id::new(&item.name);
        let popup_id = egui::Id::new(format!("{}.popup", item.name));

        collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
            .show_header(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&item.name));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let button = ui.button("Use");
                        if button.clicked() {
                            ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                        }
                        popup_below_widget(
                            ui,
                            popup_id,
                            &button,
                            egui::PopupCloseBehavior::CloseOnClickOutside,
                            |ui| {
                                ui.set_min_width(70.0);
                                ui.horizontal(|ui| {
                                    DragValue::new(&mut self.use_num).ui(ui);
                                    if ui.button("Done").clicked() {
                                        println!("Used item");
                                    }
                                })
                            },
                        );

                        ui.label(
                            RichText::new(format!("x{}", item.count))
                                .color(Color32::LIGHT_GREEN)
                                .italics(),
                        );
                    })
                })
            })
            .body(|ui| {
                ui.label(&item.description);

                let flavor = RichText::new(format!("\"{}\"", &item.flavor_text)).italics();

                ui.label(flavor);
            });
    }
}

impl DndTabImpl for Character {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &crate::state::DndState,
        tx: &message_io::events::EventSender<crate::listener::Signal>,
        rx: &std::sync::mpsc::Receiver<common::message::DndMessage>,
    ) {
        TopBottomPanel::top("stats")
            .min_height(100.0)
            .resizable(false)
            .show_inside(ui, |ui| {
                ui.heading("Gleebo");
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    StatWidget::new("CHR", 10).ui(ui);
                    StatWidget::new("STR", 10).ui(ui);
                    StatWidget::new("WIS", 10).ui(ui);
                    StatWidget::new("INT", 10).ui(ui);
                    StatWidget::new("DEX", 10).ui(ui);
                    StatWidget::new("CON", 10).ui(ui);
                });
            });

        CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("Items");
            for item in state.character.items.iter() {
                self.draw_item(ui, item);
                ui.separator();
            }
        });
    }

    fn title(&self) -> String {
        "Character".to_owned()
    }
}

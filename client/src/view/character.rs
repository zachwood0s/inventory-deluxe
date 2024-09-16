use crate::{prelude::*, state::character::commands::RefreshCharacter};
use egui::{
    collapsing_header, popup_below_widget, text::LayoutJob, tooltip_id, Button, CentralPanel,
    CollapsingHeader, Color32, DragValue, Frame, Label, Margin, Resize, RichText, TopBottomPanel,
    Vec2, Widget,
};

use crate::{
    listener::CommandQueue,
    state::{character::commands::UseItem, DndState},
};

use super::DndTabImpl;

pub struct StatWidget {
    name: String,
    value: i16,
}

impl StatWidget {
    pub fn new(name: impl ToString, value: i16) -> Self {
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

pub struct ItemWidget<'a, 'b, 'c> {
    idx: usize,
    item: Item,
    use_num: &'a mut u32,
    commands: &'b mut CommandQueue<'c>,
}

impl<'a, 'b, 'c> ItemWidget<'a, 'b, 'c> {
    fn new(
        idx: usize,
        item: Item,
        use_num: &'a mut u32,
        commands: &'b mut CommandQueue<'c>,
    ) -> Self {
        Self {
            idx,
            item,
            use_num,
            commands,
        }
    }
}

impl<'a, 'b, 'c> Widget for ItemWidget<'a, 'b, 'c> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut item_text =
            LayoutJob::single_section(self.item.name.clone(), egui::TextFormat::default());
        item_text.append(
            &format!("x{}", self.item.count),
            10.0,
            egui::TextFormat {
                color: Color32::LIGHT_GREEN,
                italics: true,
                ..Default::default()
            },
        );

        let id = egui::Id::new(&self.item.name);
        let popup_id = egui::Id::new(format!("{}.popup", self.item.name));

        collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
            .show_header(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&self.item.name));
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
                                    DragValue::new(self.use_num)
                                        .range(1..=self.item.count)
                                        .ui(ui);

                                    if ui.button("Done").clicked() {
                                        self.commands.add(UseItem::new(self.idx, *self.use_num));

                                        ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                                    }
                                })
                            },
                        );

                        ui.label(
                            RichText::new(format!("x{}", self.item.count))
                                .color(Color32::LIGHT_GREEN)
                                .italics(),
                        );
                    })
                })
            })
            .body(|ui| {
                ui.label(&self.item.description);

                let flavor = RichText::new(format!("\"{}\"", &self.item.flavor_text)).italics();

                ui.label(flavor);
            })
            .0
    }
}

#[derive(Default)]
pub struct Character {
    use_num: u32,
}

impl DndTabImpl for Character {
    fn ui(&mut self, ui: &mut egui::Ui, state: &DndState, commands: &mut CommandQueue) {
        TopBottomPanel::top("stats")
            .min_height(100.0)
            .resizable(false)
            .show_inside(ui, |ui| {
                let char = &state.character.character;

                ui.horizontal(|ui| {
                    let refresh = egui::include_image!("../../assets/refresh.png");

                    let image = egui::Image::new(refresh).fit_to_exact_size(Vec2::new(20.0, 20.0));
                    if Button::image(image)
                        .fill(Color32::TRANSPARENT)
                        .ui(ui)
                        .clicked()
                    {
                        commands.add(RefreshCharacter);
                    }

                    ui.heading(&char.name);
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    StatWidget::new("CHR", char.chr).ui(ui);
                    StatWidget::new("STR", char.str).ui(ui);
                    StatWidget::new("WIS", char.wis).ui(ui);
                    StatWidget::new("INT", char.int).ui(ui);
                    StatWidget::new("DEX", char.dex).ui(ui);
                    StatWidget::new("CON", char.con).ui(ui);
                });
            });

        CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("Items");
            for (idx, item) in state.character.items.iter().enumerate() {
                ItemWidget::new(idx, item.clone(), &mut self.use_num, commands).ui(ui);
                ui.separator();
            }
        });
    }

    fn title(&self) -> String {
        "Character".to_owned()
    }
}

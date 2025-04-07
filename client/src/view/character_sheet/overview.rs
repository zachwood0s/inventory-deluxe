use common::Character;
use egui::{CentralPanel, Image, TopBottomPanel, Widget, Window};
use egui_extras::{Size, Strip, StripBuilder};

pub struct CharacterSheetWindow<'a> {
    pub sheet: CharacterSheet<'a>,
}

impl CharacterSheetWindow<'_> {
    pub fn ui(self, ui: &mut egui::Ui) {
        Window::new("Character")
            .title_bar(false)
            .default_open(true)
            .show(ui.ctx(), |ui| {
                self.sheet.ui(ui);
            });
    }
}

pub struct CharacterSheet<'a> {
    character: &'a Character,
}

impl<'a> CharacterSheet<'a> {
    pub fn new(character: &'a Character) -> Self {
        Self { character }
    }

    pub fn ui(self, ui: &mut egui::Ui) {
        TopBottomPanel::top("top_bar").min_height(50.0).resizable(false).show_inside(ui, |ui| {
            StripBuilder::new(ui).size(Size::exact(50.0)).size(Size::remainder()).horizontal(|mut strip| {
                strip.cell(|ui| {
                    Image::new("https://cdn.discordapp.com/attachments/1295543267231928321/1295557362551230514/th.png?ex=67f52311&is=67f3d191&hm=663ceb5f04136e4456ee988b8c97879afa1d40c98b85b7bb2d5075b418ec9420&").ui(ui);
                });
                strip.cell(|ui| {
                    ui.label("Remainder");
                });

            })
        });

        TopBottomPanel::bottom("bottom_bar")
            .min_height(10.0)
            .resizable(false)
            .show_inside(ui, |ui| {
                ui.label("bottom");
            });

        CentralPanel::default().show_inside(ui, |ui| {
            ui.label("Remainder");
        });
    }
}

use core::f32;
use std::ops::RangeInclusive;

use common::board::{BoardPiece, BoardPieceData, PlayerPieceData};
use egui::{DragValue, Margin, Pos2, Rect, RichText, Style, TextEdit, Ui, WidgetText, Window};

use crate::state::DndState;

pub trait PropertiesDisplay {
    fn display_props(&mut self, ui: &mut Ui, state: &DndState);
}

impl<T> PropertiesDisplay for &mut T
where
    T: PropertiesDisplay,
{
    fn display_props(&mut self, ui: &mut Ui, state: &DndState) {
        (*self).display_props(ui, state)
    }
}

impl PropertiesDisplay for BoardPiece {
    fn display_props(&mut self, ui: &mut Ui, state: &DndState) {
        let title = RichText::new("Properties").text_style(egui::TextStyle::Body);
        let frame =
            egui::Frame::window(&Style::default()).inner_margin(Margin::symmetric(6.0, 4.0));

        Window::new(title)
            .resizable(false)
            .collapsible(false)
            .frame(frame)
            .show(ui.ctx(), |ui| {
                ui.set_width(200.0);

                ui.collapsing(
                    format!("{} General", egui_phosphor::regular::SLIDERS),
                    |ui| {
                        egui::Grid::new("general").num_columns(3).show(ui, |ui| {
                            LabeledRow("Name", &mut self.name).display_props(ui, state);
                            self.rect.display_props(ui, state);
                            LabeledRow("Sorting Layer", &mut self.sorting_layer)
                                .display_props(ui, state);
                            LabeledRow("Piece Type", &mut PieceSelector(&mut self.data))
                                .display_props(ui, state);
                        });
                    },
                );

                ui.collapsing(format!("{} Display", egui_phosphor::regular::IMAGE), |ui| {
                    egui::Grid::new("general").num_columns(3).show(ui, |ui| {
                        LabeledRow("Image", &mut self.image_url).display_props(ui, state);
                        LabeledRow("Color", &mut self.color).display_props(ui, state);
                    });
                });

                ui.collapsing(
                    format!("{} Grid", egui_phosphor::regular::DOTS_NINE),
                    |ui| {
                        egui::Grid::new("general").num_columns(3).show(ui, |ui| {
                            self.snap_to_grid.display_props(ui, state);
                            LabeledRow("Locked", &mut self.locked).display_props(ui, state);
                        });
                    },
                );

                match &mut self.data {
                    BoardPieceData::Player(data) => data.display_props(ui, state),
                    BoardPieceData::None => {}
                }
            });
    }
}

impl PropertiesDisplay for PlayerPieceData {
    fn display_props(&mut self, ui: &mut Ui, state: &DndState) {
        ui.collapsing(format!("{} Player", egui_phosphor::regular::PERSON), |ui| {
            egui::Grid::new("general").num_columns(3).show(ui, |ui| {
                LabeledRow(
                    "Link Stats",
                    &mut PlayerNameSelector(&mut self.link_stats_to),
                )
                .display_props(ui, state);
            });
        });
    }
}

const NUM_INPUT_WIDTH: f32 = 60.0;

impl PropertiesDisplay for Rect {
    fn display_props(&mut self, ui: &mut Ui, state: &DndState) {
        let mut new_min = self.min;
        let mut dims = self.size().to_pos2();

        LabeledRow("Position", &mut RangedPos(&mut new_min, -1000.0..=1000.0))
            .display_props(ui, state);
        LabeledRow("Dimensions", &mut RangedPos(&mut dims, 0.5..=100.0)).display_props(ui, state);

        *self = self.translate(new_min - self.min);
        self.set_width(dims.x);
        self.set_height(dims.y);
    }
}

struct LabeledRow<'a, L, T>(L, &'a mut T)
where
    T: PropertiesDisplay,
    L: Into<WidgetText> + Copy;

impl<L: Copy, T> PropertiesDisplay for LabeledRow<'_, L, T>
where
    T: PropertiesDisplay,
    L: Into<WidgetText> + Copy,
{
    fn display_props(&mut self, ui: &mut Ui, state: &DndState) {
        ui.label(self.0);
        self.1.display_props(ui, state);
        ui.end_row();
    }
}

struct RangedNum<'a, Num>(&'a mut Num, RangeInclusive<Num>)
where
    Num: emath::Numeric;

impl<Num> PropertiesDisplay for RangedNum<'_, Num>
where
    Num: emath::Numeric,
{
    fn display_props(&mut self, ui: &mut Ui, _: &DndState) {
        let Self(val, range) = self;
        ui.add_sized(
            [NUM_INPUT_WIDTH, 20.0],
            DragValue::new(*val).range(range.clone()),
        );
    }
}

struct RangedPos<'a>(&'a mut Pos2, RangeInclusive<f32>);

impl PropertiesDisplay for RangedPos<'_> {
    fn display_props(&mut self, ui: &mut Ui, state: &DndState) {
        let Self(pos, range) = self;

        ui.horizontal(|ui| {
            RangedNum(&mut pos.x, range.clone()).display_props(ui, state);
            RangedNum(&mut pos.y, range.clone()).display_props(ui, state);
        });
    }
}

impl PropertiesDisplay for Pos2 {
    fn display_props(&mut self, ui: &mut Ui, state: &DndState) {
        RangedPos(self, f32::NEG_INFINITY..=f32::INFINITY).display_props(ui, state);
    }
}

impl PropertiesDisplay for common::board::SortingLayer {
    fn display_props(&mut self, ui: &mut Ui, state: &DndState) {
        RangedNum(&mut self.0, 0..=10).display_props(ui, state);
    }
}

impl PropertiesDisplay for common::board::GridSnap {
    fn display_props(&mut self, ui: &mut Ui, state: &DndState) {
        let mut is_snap = self.is_snap();

        LabeledRow("Snap to Grid", &mut is_snap).display_props(ui, state);

        match self {
            common::board::GridSnap::MajorSpacing(x) if is_snap => {
                LabeledRow("Major Spacing", &mut RangedNum(x, 0.5..=10.0)).display_props(ui, state);
            }
            common::board::GridSnap::None if is_snap => {
                // On this frame just set the spacing, next frame the
                // properties will actually display since the above branch is taken
                *self = common::board::GridSnap::MajorSpacing(1.0);
            }
            _ => *self = common::board::GridSnap::None,
        }
    }
}

impl PropertiesDisplay for String {
    fn display_props(&mut self, ui: &mut Ui, _: &DndState) {
        let width = NUM_INPUT_WIDTH * 2.0 + ui.spacing().item_spacing.x;
        ui.add_sized([width, 20.0], TextEdit::singleline(self));
    }
}

impl PropertiesDisplay for common::board::Color {
    fn display_props(&mut self, ui: &mut Ui, _: &DndState) {
        ui.color_edit_button_rgba_unmultiplied(&mut *self);
    }
}

impl PropertiesDisplay for bool {
    fn display_props(&mut self, ui: &mut Ui, _: &DndState) {
        ui.checkbox(&mut *self, "");
    }
}

struct PieceSelector<'a>(&'a mut BoardPieceData);
impl PropertiesDisplay for PieceSelector<'_> {
    fn display_props(&mut self, ui: &mut Ui, _: &DndState) {
        egui::ComboBox::from_id_salt("type_selection")
            .selected_text(format!("{}", self.0))
            .show_ui(ui, |ui| {
                ui.selectable_value(self.0, BoardPieceData::None, "None");
                ui.selectable_value(
                    self.0,
                    BoardPieceData::Player(PlayerPieceData::default()),
                    "Player",
                );
            });
    }
}

struct PlayerNameSelector<'a>(&'a mut Option<String>);
impl PropertiesDisplay for PlayerNameSelector<'_> {
    fn display_props(&mut self, ui: &mut Ui, state: &DndState) {
        let default = String::from("None");
        let selected_text = self.0.as_ref().unwrap_or(&default);
        egui::ComboBox::from_id_salt("stats_from_selection")
            .selected_text(selected_text)
            .show_ui(ui, |ui| {
                ui.selectable_value(self.0, None, "None");
                for character in state.character_list.iter() {
                    ui.selectable_value(self.0, Some(character.clone()), character);
                }
            });
    }
}

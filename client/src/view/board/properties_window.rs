use core::f32;
use std::ops::RangeInclusive;

use common::board::{BoardPiece, BoardPieceData, CharacterPieceData};
use egui::{
    Align, Button, CentralPanel, DragValue, Label, Margin, Pos2, Rect, RichText, Rounding, Style,
    TextEdit, Ui, Widget, WidgetText, Window,
};
use egui_extras::{Size, Strip, StripBuilder};

use crate::state::DndState;

pub struct PropertiesCtx<'a> {
    pub state: &'a DndState,
    pub open: &'a mut bool,
    pub changed: bool,
}

pub trait PropertiesDisplay {
    fn display_props(&mut self, ui: &mut Ui, ctx: &mut PropertiesCtx);
}

impl<T> PropertiesDisplay for &mut T
where
    T: PropertiesDisplay,
{
    fn display_props(&mut self, ui: &mut Ui, ctx: &mut PropertiesCtx) {
        (*self).display_props(ui, ctx)
    }
}

impl PropertiesDisplay for Option<&mut BoardPiece> {
    fn display_props(&mut self, ui: &mut Ui, ctx: &mut PropertiesCtx) {
        let title = RichText::new("Properties").text_style(egui::TextStyle::Body);
        let frame =
            egui::Frame::window(&Style::default()).inner_margin(Margin::symmetric(6.0, 4.0));

        let mut open = *ctx.open;
        Window::new(title)
            .open(&mut open)
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .order(egui::Order::Foreground)
            .frame(frame)
            .show(ui.ctx(), |ui| {
                ui.set_width(230.0);

                ui.horizontal(|ui| {
                    StripBuilder::new(ui)
                        .size(Size::remainder())
                        .size(Size::exact(20.0))
                        .horizontal(|mut strip| {
                            strip.cell(|ui| {
                                ui.label("Properties");
                            });
                            strip.cell(|ui| {
                                if Button::new(egui_phosphor::regular::X)
                                    .frame(false)
                                    .ui(ui)
                                    .clicked()
                                {
                                    *ctx.open = false;
                                }
                            })
                        });
                });
                ui.separator();

                let Some(piece) = self else {
                    ui.label("Try adding a piece !whydontya!");
                    return;
                };

                ui.collapsing(
                    format!("{} General", egui_phosphor::regular::SLIDERS),
                    |ui| {
                        egui::Grid::new("general").num_columns(3).show(ui, |ui| {
                            LabeledRow("Name", &mut piece.name).display_props(ui, ctx);
                            piece.rect.display_props(ui, ctx);
                            LabeledRow("Sorting Layer", &mut piece.sorting_layer)
                                .display_props(ui, ctx);
                            LabeledRow("Piece Type", &mut PieceSelector(&mut piece.data))
                                .display_props(ui, ctx);
                        });
                    },
                );

                ui.collapsing(format!("{} Display", egui_phosphor::regular::IMAGE), |ui| {
                    egui::Grid::new("general").num_columns(3).show(ui, |ui| {
                        LabeledRow("Image", &mut piece.image_url).display_props(ui, ctx);
                        LabeledRow("Color", &mut piece.color).display_props(ui, ctx);
                    });
                });

                ui.collapsing(
                    format!("{} Grid", egui_phosphor::regular::DOTS_NINE),
                    |ui| {
                        egui::Grid::new("general").num_columns(3).show(ui, |ui| {
                            piece.snap_to_grid.display_props(ui, ctx);
                            LabeledRow("Locked", &mut piece.locked).display_props(ui, ctx);
                        });
                    },
                );

                match &mut piece.data {
                    BoardPieceData::Character(data) => {
                        ui.collapsing(
                            format!("{} Character", egui_phosphor::regular::PERSON),
                            |ui| {
                                egui::Grid::new("general").num_columns(3).show(ui, |ui| {
                                    LabeledRow("Display Name", &mut piece.display_name)
                                        .display_props(ui, ctx);

                                    LabeledRow(
                                        "Link Stats",
                                        &mut PlayerNameSelector(&mut data.link_stats_to),
                                    )
                                    .display_props(ui, ctx);
                                });
                            },
                        );
                    }
                    BoardPieceData::None => {}
                }
            });
    }
}

const NUM_INPUT_WIDTH: f32 = 60.0;

impl PropertiesDisplay for Rect {
    fn display_props(&mut self, ui: &mut Ui, ctx: &mut PropertiesCtx) {
        let mut new_min = self.min;
        let mut dims = self.size().to_pos2();

        LabeledRow("Position", &mut RangedPos(&mut new_min, -1000.0..=1000.0))
            .display_props(ui, ctx);
        LabeledRow("Dimensions", &mut RangedPos(&mut dims, 0.5..=100.0)).display_props(ui, ctx);

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
    fn display_props(&mut self, ui: &mut Ui, ctx: &mut PropertiesCtx) {
        ui.label(self.0);
        self.1.display_props(ui, ctx);
        ui.end_row();
    }
}

struct RangedPos<'a>(&'a mut Pos2, RangeInclusive<f32>);

impl PropertiesDisplay for RangedPos<'_> {
    fn display_props(&mut self, ui: &mut Ui, ctx: &mut PropertiesCtx) {
        let Self(pos, range) = self;

        ui.horizontal(|ui| {
            RangedNum(&mut pos.x, range.clone()).display_props(ui, ctx);
            RangedNum(&mut pos.y, range.clone()).display_props(ui, ctx);
        });
    }
}

impl PropertiesDisplay for Pos2 {
    fn display_props(&mut self, ui: &mut Ui, ctx: &mut PropertiesCtx) {
        RangedPos(self, f32::NEG_INFINITY..=f32::INFINITY).display_props(ui, ctx);
    }
}

impl PropertiesDisplay for common::board::SortingLayer {
    fn display_props(&mut self, ui: &mut Ui, ctx: &mut PropertiesCtx) {
        RangedNum(&mut self.0, 0..=10).display_props(ui, ctx);
    }
}

impl PropertiesDisplay for common::board::GridSnap {
    fn display_props(&mut self, ui: &mut Ui, ctx: &mut PropertiesCtx) {
        let mut is_snap = self.is_snap();

        LabeledRow("Snap to Grid", &mut is_snap).display_props(ui, ctx);

        match self {
            common::board::GridSnap::MajorSpacing(x) if is_snap => {
                LabeledRow("Major Spacing", &mut RangedNum(x, 0.5..=10.0)).display_props(ui, ctx);
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
    fn display_props(&mut self, ui: &mut Ui, ctx: &mut PropertiesCtx) {
        let width = NUM_INPUT_WIDTH * 2.0 + ui.spacing().item_spacing.x;
        ctx.changed |= ui
            .add_sized([width, 20.0], TextEdit::singleline(self))
            .changed();
    }
}

impl PropertiesDisplay for common::board::Color {
    fn display_props(&mut self, ui: &mut Ui, ctx: &mut PropertiesCtx) {
        ctx.changed |= ui.color_edit_button_rgba_unmultiplied(&mut *self).changed()
    }
}

impl PropertiesDisplay for bool {
    fn display_props(&mut self, ui: &mut Ui, ctx: &mut PropertiesCtx) {
        ctx.changed |= ui.checkbox(&mut *self, "").changed();
    }
}

struct PieceSelector<'a>(&'a mut BoardPieceData);
impl PropertiesDisplay for PieceSelector<'_> {
    fn display_props(&mut self, ui: &mut Ui, ctx: &mut PropertiesCtx) {
        let mut before_value = self.0.clone();

        egui::ComboBox::from_id_salt("type_selection")
            .selected_text(format!("{}", self.0))
            .show_ui(ui, |ui| {
                ui.selectable_value(self.0, BoardPieceData::None, "None");
                ui.selectable_value(
                    self.0,
                    BoardPieceData::Character(CharacterPieceData::default()),
                    "Character",
                );
            });

        ctx.changed |= &mut before_value != self.0;
    }
}

struct PlayerNameSelector<'a>(&'a mut Option<String>);
impl PropertiesDisplay for PlayerNameSelector<'_> {
    fn display_props(&mut self, ui: &mut Ui, ctx: &mut PropertiesCtx) {
        let mut before_value = self.0.clone();

        let default = String::from("None");
        let selected_text = self.0.as_ref().unwrap_or(&default);

        egui::ComboBox::from_id_salt("stats_from_selection")
            .selected_text(selected_text)
            .show_ui(ui, |ui| {
                ui.selectable_value(self.0, None, "None");
                for character in ctx.state.character_list.iter() {
                    ui.selectable_value(self.0, Some(character.clone()), character);
                }
            });

        ctx.changed |= &mut before_value != self.0
    }
}

struct RangedNum<'a, Num>(&'a mut Num, RangeInclusive<Num>)
where
    Num: emath::Numeric;

impl<Num> PropertiesDisplay for RangedNum<'_, Num>
where
    Num: emath::Numeric,
{
    fn display_props(&mut self, ui: &mut Ui, ctx: &mut PropertiesCtx) {
        let Self(val, range) = self;
        let resp = ui.add_sized(
            [NUM_INPUT_WIDTH, 20.0],
            DragValue::new(*val).range(range.clone()),
        );

        if resp.changed() || resp.drag_stopped() {
            ctx.changed |= !resp.dragged();
        }
    }
}

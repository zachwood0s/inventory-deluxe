use egui::{CornerRadius, InnerResponse, Margin, Rounding, Style, Ui};

pub struct Emphasized;

impl Emphasized {
    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let style = Style::default();

        egui::Frame::NONE
            .fill(style.visuals.extreme_bg_color)
            .corner_radius(CornerRadius::from(5.0))
            .inner_margin(Margin::from(5))
            .outer_margin(Margin::symmetric(0, 2))
            .show(ui, add_contents)
    }
}

use egui::{Button, Widget};

pub struct ToggleIcon<'a> {
    toggle_value: &'a mut bool,
    on_icon: &'a str,
    off_icon: &'a str,
    disable_icon: &'a str,
    hover_tooltip: Option<&'static str>,
}

impl<'a> ToggleIcon<'a> {
    pub fn new(
        toggle_value: &'a mut bool,
        on_icon: &'a str,
        off_icon: &'a str,
        disable_icon: &'a str,
    ) -> Self {
        Self {
            toggle_value,
            on_icon,
            off_icon,
            disable_icon,
            hover_tooltip: None,
        }
    }

    pub fn hover(mut self, hover_tooltip: &'static str) -> Self {
        self.hover_tooltip = Some(hover_tooltip);
        self
    }
}

impl Widget for ToggleIcon<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let icon = if !ui.is_enabled() {
            self.disable_icon
        } else if *self.toggle_value {
            self.on_icon
        } else {
            self.off_icon
        };

        let mut resp = ui.add(Button::new(icon));

        if let Some(hover_tooltip) = self.hover_tooltip {
            resp = resp.on_hover_text(hover_tooltip);
        }

        if resp.clicked() {
            *self.toggle_value = !*self.toggle_value;
        }

        resp
    }
}

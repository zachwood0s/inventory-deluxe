use super::CharacterTabImpl;

#[derive(Clone)]
pub struct BiographyTab;

impl CharacterTabImpl for BiographyTab {
    fn ui(&self, ui: &mut egui::Ui, ctx: super::CharacterCtx) {
        ui.label("bio");
    }

    fn title(&self) -> &str {
        "BIOGRAPHY"
    }
}

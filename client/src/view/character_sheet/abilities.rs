use super::CharacterTabImpl;

#[derive(Clone)]
pub struct AbilitiesTab;

impl CharacterTabImpl for AbilitiesTab {
    fn ui(&self, ui: &mut egui::Ui, ctx: super::CharacterCtx) {
        ui.label("abilities");
    }

    fn title(&self) -> &str {
        "ABILITIES"
    }
}

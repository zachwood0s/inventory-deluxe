use common::{Ability, AbilityId};
use egui::{Grid, Window};
use log::{info, warn};

use crate::state::DndState;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct EditState {
    ability: Ability,
    id: egui::Id,
}

impl EditState {
    fn load(ui: &mut egui::Ui, ability_id: &AbilityId, state: &DndState) -> Option<Self> {
        let id = ui.make_persistent_id("edit").with(ability_id);

        // TODO: This copies the whole ability every frame, might not be ideal
        // May want to arc/mutex this so that the clone is cheap
        ui.data_mut(|mem| {
            mem.get_persisted(id).or_else(|| {
                let state = EditState {
                    ability: state.data.get_ability(ability_id)?.clone(),
                    id,
                };

                Some(state)
            })
        })
    }

    fn store(self, ui: &mut egui::Ui) {
        ui.data_mut(|mem| mem.insert_persisted(self.id, self))
    }
}

pub struct AbilityEdit<'a> {
    ability_id: &'a AbilityId,
    state: &'a DndState,
}

impl<'a> AbilityEdit<'a> {
    pub fn new(ability_id: &'a AbilityId, state: &'a DndState) -> Self {
        Self { ability_id, state }
    }

    pub fn show(self, ui: &mut egui::Ui) {
        let Some(mut state) = EditState::load(ui, self.ability_id, self.state) else {
            warn!("Failed to load edit state for ability: {}", self.ability_id);
            return;
        };

        Grid::new("ability_grid").num_columns(2).show(ui, |ui| {
            ui.label("Name");
            ui.text_edit_singleline(&mut *state.ability.name);
            ui.end_row();

            ui.label("Description");
            ui.text_edit_multiline(&mut state.ability.description);
            ui.end_row();

            ui.label("Ability Type");
            ui.text_edit_singleline(&mut state.ability.ability_type);
            ui.end_row();

            ui.label("Resource");
            ui.text_edit_singleline(&mut state.ability.resource);
            ui.end_row();
            /*
                     *
            pub notes: Option<String>,
            pub ability_type: String,
            pub flavor_text: Option<String>,
            pub resource: String,
            pub max_count: i64,
                     * */
        });

        if ui.button("Save").clicked() {
            info!("Save");
        }

        state.store(ui);
    }
}

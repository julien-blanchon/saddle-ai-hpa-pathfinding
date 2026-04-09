use saddle_bevy_e2e::action::Action;

use crate::LabDiagnostics;

pub(super) fn wait_for_dynamic_replan(max_frames: u32) -> Action {
    Action::WaitUntil {
        label: "dynamic route replanned".into(),
        condition: Box::new(|world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            diagnostics.dynamic_cost_after > diagnostics.dynamic_cost_before
        }),
        max_frames,
    }
}

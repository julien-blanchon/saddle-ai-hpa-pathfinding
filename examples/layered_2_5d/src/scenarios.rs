use saddle_bevy_e2e::{action::Action, actions::assertions, scenario::Scenario};

pub fn list() -> Vec<&'static str> {
    vec!["visual_check"]
}

pub fn by_name(name: &str) -> Option<Scenario> {
    match name {
        "visual_check" => Some(visual_check()),
        _ => None,
    }
}

fn log_path_stats() -> Action {
    Action::Custom(Box::new(|world| {
        let pane = world.resource::<saddle_ai_hpa_pathfinding_example_support::HpaExamplePane>();
        bevy::log::info!(
            "[e2e] path stats: corridor_len={}, waypoint_count={}, total_cost={:.2}",
            pane.corridor_len, pane.waypoint_count, pane.total_cost
        );
    }))
}

fn visual_check() -> Scenario {
    Scenario::builder("visual_check")
        .description("Verify cross-layer path renders through the stair transition.")
        .then(Action::WaitFrames(90))
        .then(Action::Log("layered path settled".into()))
        .then(log_path_stats())
        .then(Action::Screenshot("cross_layer_path".into()))
        .then(Action::WaitFrames(2))
        .then(assertions::log_summary("layered_visual_check"))
        .build()
}

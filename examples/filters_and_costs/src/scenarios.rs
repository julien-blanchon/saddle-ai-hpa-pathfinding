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
            "[e2e] filter path stats: corridor_len={}, waypoint_count={}, total_cost={:.2}",
            pane.corridor_len, pane.waypoint_count, pane.total_cost
        );
    }))
}

fn visual_check() -> Scenario {
    Scenario::builder("visual_check")
        .description("Verify two agents with different filter profiles produce visible, distinct paths.")
        .then(Action::WaitFrames(90))
        .then(Action::Log("filter paths settled".into()))
        .then(log_path_stats())
        .then(Action::Screenshot("two_filters".into()))
        .then(Action::WaitFrames(2))
        .then(assertions::log_summary("filters_visual_check"))
        .build()
}

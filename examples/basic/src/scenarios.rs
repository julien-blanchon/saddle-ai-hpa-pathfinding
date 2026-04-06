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

fn enable_all_debug_layers() -> Action {
    Action::Custom(Box::new(|world| {
        let mut pane = world.resource_mut::<saddle_ai_hpa_pathfinding_example_support::HpaExamplePane>();
        pane.draw_grid = true;
        pane.draw_clusters = true;
        pane.draw_portals = true;
        pane.draw_abstract_graph = true;
        pane.draw_paths = true;
        pane.draw_heatmap = true;
    }))
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
        .description("Verify grid tiles, agent, goal, and path are visible in the basic example.")
        .then(Action::WaitFrames(90))
        .then(Action::Log("initial render settled".into()))
        .then(log_path_stats())
        .then(Action::Screenshot("initial".into()))
        .then(Action::WaitFrames(2))
        .then(enable_all_debug_layers())
        .then(Action::WaitFrames(30))
        .then(Action::Log("all debug layers enabled".into()))
        .then(Action::Screenshot("all_layers".into()))
        .then(Action::WaitFrames(2))
        .then(assertions::log_summary("basic_visual_check"))
        .build()
}

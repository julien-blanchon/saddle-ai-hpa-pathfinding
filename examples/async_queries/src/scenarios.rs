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

fn log_queue_stats() -> Action {
    Action::Custom(Box::new(|world| {
        let pane = world.resource::<saddle_ai_hpa_pathfinding_example_support::HpaExamplePane>();
        bevy::log::info!(
            "[e2e] async stats: completed_paths={}, corridor_len={}, total_cost={:.2}",
            pane.reachable_cells, pane.corridor_len, pane.total_cost
        );
    }))
}

fn visual_check() -> Scenario {
    Scenario::builder("visual_check")
        .description("Verify async query queue drains and all agent paths render.")
        .then(Action::WaitFrames(120))
        .then(Action::Log("async queue settled".into()))
        .then(log_queue_stats())
        .then(Action::Screenshot("queue_drained".into()))
        .then(Action::WaitFrames(2))
        .then(assertions::log_summary("async_queries_visual_check"))
        .build()
}

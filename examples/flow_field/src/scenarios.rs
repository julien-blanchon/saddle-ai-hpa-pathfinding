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

fn log_flow_stats() -> Action {
    Action::Custom(Box::new(|world| {
        let pane = world.resource::<saddle_ai_hpa_pathfinding_example_support::HpaExamplePane>();
        bevy::log::info!(
            "[e2e] flow field stats: reachable_cells={}",
            pane.reachable_cells
        );
    }))
}

fn visual_check() -> Scenario {
    Scenario::builder("visual_check")
        .description("Verify flow field arrows render with correct directions.")
        .then(Action::WaitFrames(90))
        .then(Action::Log("flow field settled".into()))
        .then(log_flow_stats())
        .then(Action::Screenshot("flow_arrows".into()))
        .then(Action::WaitFrames(2))
        .then(assertions::log_summary("flow_field_visual_check"))
        .build()
}

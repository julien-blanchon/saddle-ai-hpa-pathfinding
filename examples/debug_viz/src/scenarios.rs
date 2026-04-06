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

fn disable_all_except_paths() -> Action {
    Action::Custom(Box::new(|world| {
        let mut pane = world.resource_mut::<saddle_ai_hpa_pathfinding_example_support::HpaExamplePane>();
        pane.draw_grid = false;
        pane.draw_clusters = false;
        pane.draw_portals = false;
        pane.draw_abstract_graph = false;
        pane.draw_paths = true;
        pane.draw_heatmap = false;
    }))
}

fn visual_check() -> Scenario {
    Scenario::builder("visual_check")
        .description("Verify all debug visualization layers render correctly.")
        .then(Action::WaitFrames(90))
        .then(Action::Log("all layers rendering".into()))
        .then(Action::Screenshot("all_layers_on".into()))
        .then(Action::WaitFrames(2))
        .then(disable_all_except_paths())
        .then(Action::WaitFrames(30))
        .then(Action::Log("paths only".into()))
        .then(Action::Screenshot("paths_only".into()))
        .then(Action::WaitFrames(2))
        .then(assertions::log_summary("debug_viz_visual_check"))
        .build()
}

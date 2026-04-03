use bevy::prelude::*;
use saddle_bevy_e2e::{action::Action, actions::assertions, scenario::Scenario};

use crate::{LabDiagnostics, set_gate_blocked};

pub fn list_scenarios() -> Vec<&'static str> {
    vec![
        "smoke_launch",
        "hpa_pathfinding_smoke",
        "hpa_pathfinding_dynamic",
        "hpa_pathfinding_filters",
        "hpa_pathfinding_large_grid",
        "hpa_pathfinding_flow_field",
    ]
}

pub fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "smoke_launch" => Some(smoke_launch()),
        "hpa_pathfinding_smoke" => Some(hpa_pathfinding_smoke()),
        "hpa_pathfinding_dynamic" => Some(hpa_pathfinding_dynamic()),
        "hpa_pathfinding_filters" => Some(hpa_pathfinding_filters()),
        "hpa_pathfinding_large_grid" => Some(hpa_pathfinding_large_grid()),
        "hpa_pathfinding_flow_field" => Some(hpa_pathfinding_flow_field()),
        _ => None,
    }
}

fn block_gate(blocked: bool) -> Action {
    Action::Custom(Box::new(move |world| set_gate_blocked(world, blocked)))
}

fn set_flow_field_debug(enabled: bool) -> Action {
    Action::Custom(Box::new(move |world| {
        let mut pane =
            world.resource_mut::<saddle_ai_hpa_pathfinding_example_support::HpaExamplePane>();
        pane.draw_grid = enabled;
        pane.draw_heatmap = enabled;
        let mut config = world.resource_mut::<saddle_ai_hpa_pathfinding::HpaPathfindingConfig>();
        config.debug_draw_grid = enabled;
        config.debug_draw_cost_heatmap = enabled;
    }))
}

fn wait_until_smoke() -> Action {
    Action::WaitUntil {
        label: "smoke path ready".into(),
        condition: Box::new(|world| world.resource::<LabDiagnostics>().smoke_ready),
        max_frames: 120,
    }
}

fn build_smoke(name: &'static str) -> Scenario {
    Scenario::builder(name)
        .description("Boot the lab, wait for the primary path to resolve, and capture the default hierarchy/path overlay.")
        .then(wait_until_smoke())
        .then(assertions::custom("smoke path resolved", |world| {
            world.resource::<LabDiagnostics>().smoke_cost > 0.0
        }))
        .then(Action::Screenshot("smoke".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary(name))
        .build()
}

fn smoke_launch() -> Scenario {
    build_smoke("smoke_launch")
}

fn hpa_pathfinding_smoke() -> Scenario {
    build_smoke("hpa_pathfinding_smoke")
}

fn hpa_pathfinding_dynamic() -> Scenario {
    Scenario::builder("hpa_pathfinding_dynamic")
        .description("Resolve a baseline route, block the central gate, and verify invalidation plus replanning produce a more expensive path.")
        .then(Action::WaitUntil {
            label: "baseline dynamic route".into(),
            condition: Box::new(|world| world.resource::<LabDiagnostics>().dynamic_cost_before > 0.0),
            max_frames: 120,
        })
        .then(Action::Screenshot("dynamic_before".into()))
        .then(Action::WaitFrames(1))
        .then(block_gate(true))
        .then(Action::WaitUntil {
            label: "dynamic path invalidated".into(),
            condition: Box::new(|world| world.resource::<LabDiagnostics>().invalidations > 0),
            max_frames: 120,
        })
        .then(Action::WaitUntil {
            label: "dynamic path replanned".into(),
            condition: Box::new(|world| {
                let diagnostics = world.resource::<LabDiagnostics>();
                diagnostics.dynamic_cost_after > diagnostics.dynamic_cost_before
            }),
            max_frames: 180,
        })
        .then(assertions::custom("gate block increased route cost", |world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            diagnostics.dynamic_cost_after > diagnostics.dynamic_cost_before
        }))
        .then(Action::Screenshot("dynamic_after".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("hpa_pathfinding_dynamic"))
        .build()
}

fn hpa_pathfinding_filters() -> Scenario {
    Scenario::builder("hpa_pathfinding_filters")
        .description(
            "Compare two agents with different traversal profiles over the same terrain band.",
        )
        .then(Action::WaitUntil {
            label: "filter paths ready".into(),
            condition: Box::new(|world| {
                let diagnostics = world.resource::<LabDiagnostics>();
                diagnostics.wheeled_cost > 0.0 && diagnostics.utility_cost > 0.0
            }),
            max_frames: 120,
        })
        .then(assertions::custom("filter profiles diverge", |world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            diagnostics.utility_cost < diagnostics.wheeled_cost
        }))
        .then(Action::Screenshot("filters".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("hpa_pathfinding_filters"))
        .build()
}

fn hpa_pathfinding_large_grid() -> Scenario {
    Scenario::builder("hpa_pathfinding_large_grid")
        .description("Wait for the stress batch to complete and verify the queue drains without stalling the app.")
        .then(Action::WaitUntil {
            label: "stress batch complete".into(),
            condition: Box::new(|world| {
                let diagnostics = world.resource::<LabDiagnostics>();
                diagnostics.stress_completed >= 8 && diagnostics.queue_depth == 0
            }),
            max_frames: 240,
        })
        .then(assertions::custom("stress batch completed", |world| {
            world.resource::<LabDiagnostics>().stress_completed >= 8
        }))
        .then(Action::Screenshot("large_grid".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("hpa_pathfinding_large_grid"))
        .build()
}

fn hpa_pathfinding_flow_field() -> Scenario {
    Scenario::builder("hpa_pathfinding_flow_field")
        .description(
            "Build a default flow field toward the smoke goal and verify a wide-clearance variant cannot route through the single-cell gate.",
        )
        .then(wait_until_smoke())
        .then(assertions::custom("flow field reflects the gate bottleneck", |world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            diagnostics.flow_field_start_has_direction
                && diagnostics.wide_flow_field_blocked
                && diagnostics.flow_field_reachable_cells > 0
        }))
        .then(set_flow_field_debug(true))
        .then(Action::WaitFrames(8))
        .then(Action::Screenshot("flow_field".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("hpa_pathfinding_flow_field"))
        .build()
}

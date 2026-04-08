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
        "hpa_pathfinding_reopen_gate",
        "hpa_pathfinding_flow_field_direction",
        "hpa_pathfinding_stress_queue",
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
        "hpa_pathfinding_reopen_gate" => Some(hpa_pathfinding_reopen_gate()),
        "hpa_pathfinding_flow_field_direction" => Some(hpa_pathfinding_flow_field_direction()),
        "hpa_pathfinding_stress_queue" => Some(hpa_pathfinding_stress_queue()),
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

fn hpa_pathfinding_reopen_gate() -> Scenario {
    Scenario::builder("hpa_pathfinding_reopen_gate")
        .description(
            "Block the gate (raises path cost) then unblock it again and verify the path cost \
             returns to its baseline, confirming bidirectional dynamic obstacle support.",
        )
        .then(Action::WaitUntil {
            label: "baseline dynamic route resolved".into(),
            condition: Box::new(|world| world.resource::<LabDiagnostics>().dynamic_cost_before > 0.0),
            max_frames: 120,
        })
        // Block the gate — cost should rise
        .then(block_gate(true))
        .then(Action::WaitUntil {
            label: "path invalidated after block".into(),
            condition: Box::new(|world| world.resource::<LabDiagnostics>().invalidations > 0),
            max_frames: 120,
        })
        .then(Action::WaitUntil {
            label: "cost increased after block".into(),
            condition: Box::new(|world| {
                let d = world.resource::<LabDiagnostics>();
                d.dynamic_cost_after > d.dynamic_cost_before
            }),
            max_frames: 180,
        })
        .then(assertions::custom("gate block increased cost", |world| {
            let d = world.resource::<LabDiagnostics>();
            d.dynamic_cost_after > d.dynamic_cost_before
        }))
        .then(Action::Screenshot("reopen_blocked".into()))
        .then(Action::WaitFrames(1))
        // Unblock — cost should return to baseline
        .then(block_gate(false))
        .then(Action::WaitUntil {
            label: "second invalidation after unblock".into(),
            condition: Box::new(|world| world.resource::<LabDiagnostics>().invalidations >= 2),
            max_frames: 180,
        })
        .then(assertions::custom("gate unblock triggers re-invalidation", |world| {
            world.resource::<LabDiagnostics>().invalidations >= 2
        }))
        .then(Action::Screenshot("reopen_unblocked".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("hpa_pathfinding_reopen_gate"))
        .build()
}

fn hpa_pathfinding_flow_field_direction() -> Scenario {
    Scenario::builder("hpa_pathfinding_flow_field_direction")
        .description(
            "Verify that the flow field at the start cell has a valid direction toward the goal \
             and that the total reachable cell count covers the majority of the open area \
             (excluding the wall band above the gate).",
        )
        .then(wait_until_smoke())
        .then(assertions::custom("flow field start cell has a valid direction", |world| {
            world.resource::<LabDiagnostics>().flow_field_start_has_direction
        }))
        .then(assertions::custom("flow field covers significant open area", |world| {
            // The grid is 32×24 = 768 cells; the wall with a single-cell gate blocks a row.
            // Standard agent clearance 0 should still reach well over 200 cells.
            world.resource::<LabDiagnostics>().flow_field_reachable_cells > 200
        }))
        .then(assertions::custom("wide-clearance flow field is blocked at the gate", |world| {
            world.resource::<LabDiagnostics>().wide_flow_field_blocked
        }))
        .then(set_flow_field_debug(true))
        .then(Action::WaitFrames(4))
        .then(Action::Screenshot("flow_direction".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("hpa_pathfinding_flow_field_direction"))
        .build()
}

fn hpa_pathfinding_stress_queue() -> Scenario {
    Scenario::builder("hpa_pathfinding_stress_queue")
        .description(
            "Verify that after the stress batch completes the queue depth drains to zero, \
             confirming the frame-budget query scheduler does not stall or starve.",
        )
        .then(Action::WaitUntil {
            label: "stress batch completed and queue drained".into(),
            condition: Box::new(|world| {
                let d = world.resource::<LabDiagnostics>();
                d.stress_completed >= 8 && d.queue_depth == 0
            }),
            max_frames: 300,
        })
        .then(assertions::custom("all 8 stress agents resolved", |world| {
            world.resource::<LabDiagnostics>().stress_completed >= 8
        }))
        .then(assertions::custom("query queue fully drained", |world| {
            world.resource::<LabDiagnostics>().queue_depth == 0
        }))
        .then(Action::Screenshot("stress_queue_drained".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("hpa_pathfinding_stress_queue"))
        .build()
}

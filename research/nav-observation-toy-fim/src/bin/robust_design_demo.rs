use nav_observation_toy_fim::DesignCriterion;

#[path = "../correlated.rs"]
mod correlated;
#[path = "../robust.rs"]
mod robust;

use correlated::CorrelatedRangeAction;
use robust::{robust_select_correlated, ActionPerturbation, PerturbationScenario};

fn p(id: &str, delta: f64) -> ActionPerturbation {
    ActionPerturbation { action_id:id.into(), bearing_delta_rad:delta, private_sigma_scale:1.0 }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let actions=vec![
        CorrelatedRangeAction::independent("fragile-east",0.0,0.2,1),
        CorrelatedRangeAction::independent("fragile-north",90.0_f64.to_radians(),0.2,1),
        CorrelatedRangeAction::independent("stable-diagonal",45.0_f64.to_radians(),0.205,1),
        CorrelatedRangeAction::independent("stable-antidiagonal",135.0_f64.to_radians(),0.205,1),
    ];
    let scenarios=vec![
        PerturbationScenario{scenario_id:"positive".into(),actions:vec![p("fragile-east",0.35),p("fragile-north",-0.35),p("stable-diagonal",0.01),p("stable-antidiagonal",-0.01)]},
        PerturbationScenario{scenario_id:"negative".into(),actions:vec![p("fragile-east",-0.35),p("fragile-north",0.35),p("stable-diagonal",-0.01),p("stable-antidiagonal",0.01)]},
    ];
    let selected=robust_select_correlated(&actions,2,DesignCriterion::DOptimal,&scenarios,0.90)?;
    println!("ROBUST_GEOMETRY_SELECTION");
    println!("  nominal_optimum={:?}",selected.nominal_optimum.action_ids);
    println!("  selected={:?}",selected.selected_nominal.action_ids);
    println!("  nominal_information_retention={:.6}",selected.nominal_information_retention);
    println!("  worst_case_retention={:.6}",selected.worst_case_information_retention);
    println!("  worst_case_scenario={}",selected.worst_case_scenario_id);
    Ok(())
}

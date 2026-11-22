use nav_observation_toy_fim::common_cause_sequential::{
    compare_two_sensor_outcomes, enumerate_joint_branches, exact_common_cause_horizon_two,
    CommonCauseAction, JointBelief, JointOutcome,
};
use nav_observation_toy_fim::decision_value::{ActionCost, DecisionLoss, ResourceBudget};

fn action(id:&str,safe:[f64;4])->CommonCauseAction{
    CommonCauseAction{id:id.into(),cost:ActionCost{energy_units:1,latency_ticks:1,bandwidth_units:1},acquisition_success:[1.0;4],outcomes:vec![
        JointOutcome{id:"safe-reading".into(),likelihood:safe},
        JointOutcome{id:"unsafe-reading".into(),likelihood:safe.map(|p|1.0-p)}
    ]}
}
fn main()->Result<(),Box<dyn std::error::Error>>{
    let prior=JointBelief::new([0.45,0.05,0.45,0.05])?;
    let uwb=action("uwb",[0.05,0.90,0.95,0.95]);
    let camera=action("camera",[0.08,0.88,0.90,0.92]);
    let imu=action("imu-motion",[0.15,0.35,0.85,0.75]);
    let dependence=compare_two_sensor_outcomes(prior,&uwb,"safe-reading",&camera,"safe-reading")?;
    println!("P_both_safe_given_unsafe_actual={:.6}",dependence.actual_probability_given_state_zero);
    println!("P_both_safe_given_unsafe_independent={:.6}",dependence.independent_probability_given_state_zero);
    println!("joint_dependence_ratio={:.3}",dependence.actual_probability_given_state_zero/dependence.independent_probability_given_state_zero);
    for branch in enumerate_joint_branches(&uwb,prior)?{
        println!("uwb_branch={} p={:.6} posterior_common_cause={:.6} posterior_state_one={:.6}",branch.branch_id,branch.probability,branch.posterior.p_common_cause(),branch.posterior.p_state_one());
    }
    let budget=ResourceBudget{energy_units:2,latency_ticks:2,bandwidth_units:2};
    let plan=exact_common_cause_horizon_two(&[uwb,camera,imu],prior,budget,DecisionLoss::zero_one())?.ok_or("no positive value plan")?;
    println!("exact_first_action={}",plan.first_action);
    println!("expected_terminal_risk={:.6}",plan.expected_terminal_risk);
    for b in plan.branches{println!("branch={} P(C=1)={:.6} next={}",b.branch_id,b.posterior_common_cause,b.second_action.as_deref().unwrap_or("stop"));}
    println!("scope=synthetic_four_state_common_cause_model_not_physical_navigation_qualification");
    Ok(())
}

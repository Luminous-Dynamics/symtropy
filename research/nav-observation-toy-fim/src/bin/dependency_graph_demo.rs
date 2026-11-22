use nav_observation_toy_fim::dependency_graph::{
    compare_source_outcomes_given_state_zero, exact_domain_horizon_two, DomainAction,
    DomainOutcome, FailureDomainModel,
};
use nav_observation_toy_fim::decision_value::{ActionCost,DecisionLoss,ResourceBudget};

fn action(id:&str,domains:Vec<&str>,kind:&str,model:&FailureDomainModel)->DomainAction{
    let mut safe=Vec::new();
    for state in 0..2{for mask in 0..model.cause_pattern_count(){
        let clock=mask&1!=0;let power=mask&2!=0;
        safe.push(match(kind,state,power,clock){
            ("power",0,false,_)=>0.05,("power",0,true,_)=>0.90,("power",1,_,_)=>0.95,
            ("clock",0,_,false)=>0.15,("clock",0,_,true)=>0.35,("clock",1,_,false)=>0.85,("clock",1,_,true)=>0.75,
            ("mixed",0,false,false)=>0.08,("mixed",0,true,_)=>0.88,("mixed",0,false,true)=>0.10,("mixed",1,_,_)=>0.92,_=>0.5
        });
    }}
    let n=model.joint_state_count();
    DomainAction{id:id.into(),cost:ActionCost{energy_units:1,latency_ticks:1,bandwidth_units:1},failure_domains:domains.into_iter().map(str::to_owned).collect(),acquisition_success:vec![1.0;n],outcomes:vec![DomainOutcome{id:"safe-reading".into(),likelihood:safe.clone()},DomainOutcome{id:"unsafe-reading".into(),likelihood:safe.iter().map(|p|1.0-p).collect()}]}
}
fn main()->Result<(),Box<dyn std::error::Error>>{
    let cause_prior=[0.81,0.09,0.09,0.01];let mut prior=Vec::new();for _ in 0..2{prior.extend(cause_prior.map(|p|p*0.5));}
    let model=FailureDomainModel::new(vec!["clock".into(),"power".into()],prior)?;
    let actions=vec![action("uwb",vec!["power"],"power",&model),action("camera",vec!["power"],"mixed",&model),action("imu",vec!["clock"],"clock",&model)];
    let(shared,independent)=compare_source_outcomes_given_state_zero(&model,&model.prior,&actions[0],"safe-reading",&actions[1],"safe-reading")?;
    let(disjoint,disjoint_independent)=compare_source_outcomes_given_state_zero(&model,&model.prior,&actions[0],"safe-reading",&actions[2],"safe-reading")?;
    println!("shared_power_actual_joint={:.6} naive_independent={:.6} ratio={:.3}",shared,independent,shared/independent);
    println!("disjoint_power_clock_actual_joint={:.6} product_of_marginals={:.6}",disjoint,disjoint_independent);
    let plan=exact_domain_horizon_two(&actions,&model,&model.prior,ResourceBudget{energy_units:2,latency_ticks:2,bandwidth_units:2},DecisionLoss::zero_one())?.ok_or("no positive plan")?;
    println!("exact_first_action={} expected_terminal_risk={:.6}",plan.first_action,plan.expected_terminal_risk);
    for b in plan.branches{println!("branch={} P(service_one)={:.6} causes={:?} next={}",b.branch_id,b.posterior_service_one,b.posterior_failure_domains,b.second_action.as_deref().unwrap_or("stop"));}
    println!("scope=two_failure_domains_exact_small_instance_not_physical_dependency_proof");
    Ok(())
}

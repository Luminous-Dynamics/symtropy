use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

const CLAIM: &str = "lifecycle_action_profile";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: String, pub claim_ceiling: String, pub functional_unit: String,
    pub scope_id: String, pub asset_mass_g: u64, pub horizon_cycles: usize,
    pub service_hours_per_cycle: u64, pub minimum_condition_bp: u32,
    pub degradation_per_cycle_bp: u32, pub maximum_repairs: u32,
    pub initial_condition_bp: u32, pub initial_lineage_share_bp: u32,
    pub evidence: Vec<Evidence>, pub actions: Vec<Action>, pub scenarios: Vec<Scenario>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub id: String, pub payload: String, pub sha256: String,
    pub status: EvidenceStatus, pub claim_id: String, pub scope_id: String, pub coverage: Coverage,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub id: String, pub kind: Kind, pub evidence_id: String, pub safety: Gate,
    pub function: Gate, pub inspection: Gate, pub recovery_bp: u32,
    pub lineage_after_bp: Option<u32>, pub retired_mass_g: u64,
    pub fates: Fates, pub metrics: Metrics,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct Metrics {
    pub cost_cents: u64, pub virgin_material_g: u64, pub energy_wh: u64,
    pub ghg_co2e_g: u64, pub water_ml: u64, pub residual_waste_g: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct Fates {
    pub reused_component_g: u64, pub recycled_material_g: u64,
    pub energy_recovery_g: u64, pub residual_g: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Scenario { pub id: String, pub plans: Vec<Plan>, pub expected_frontier: Vec<String> }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Plan { pub id: String, pub steps: Vec<String>, pub expected: Expected }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Expected {
    pub disposition: Disposition, pub delivered_service_hours: u64,
    pub lineage_weighted_service_hours: u64, pub condition_after_cycle_bp: Vec<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus { Valid, Stale, Revoked, Unknown }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Coverage { Complete, Partial, Unknown }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Gate { Pass, Fail, Unknown }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Kind { Continue, Repair, Remanufacture, ReplaceRecycle, ReplaceEnergyRecovery, ReplaceMixed }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    Eligible, InvalidPlan, InvalidDigest, RejectedEvidence, OutOfScope,
    BlockedIncompleteCoverage, BlockedUnknown, RejectedHardGate,
    RejectedCondition, RejectedRepairLimit, RejectedMassBalance, InvalidMetrics,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Totals {
    pub cost_cents: u64, pub virgin_material_g: u64, pub energy_wh: u64,
    pub ghg_co2e_g: u64, pub water_ml: u64, pub residual_waste_g: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct FateTotals {
    pub retired_mass_g: u64, pub reused_component_g: u64, pub recycled_material_g: u64,
    pub energy_recovery_g: u64, pub residual_g: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanResult {
    pub scenario_id: String, pub plan_id: String, pub disposition: Disposition, pub reason: String,
    pub complete_service_horizon: bool, pub delivered_service_hours: u64,
    pub lineage_weighted_service_hours: u64, pub condition_after_cycle_bp: Vec<u32>,
    pub final_condition_bp: u32, pub final_lineage_share_bp: u32, pub repairs_used: u32,
    pub metrics: Totals, pub fates: FateTotals,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScenarioResult {
    pub scenario_id: String, pub plans: Vec<PlanResult>, pub eligible_plan_ids: Vec<String>,
    pub pareto_frontier: Vec<String>, pub oracle_frontier: Vec<String>,
    pub transition_oracle_agreement: bool, pub frontier_oracle_agreement: bool,
    pub fixture_expectations_match: bool, pub status: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Report {
    pub tool_version: String, pub schema_version: String, pub source_revision: String,
    pub manifest_sha256: String, pub claim_ceiling: String, pub functional_unit: String,
    pub asset_mass_g: u64, pub horizon_cycles: usize, pub evidence_count: usize,
    pub action_count: usize, pub scenario_count: usize, pub plan_count: usize,
    pub all_evidence_digests_valid: bool, pub scenarios: Vec<ScenarioResult>,
    pub overall_status: String,
}
#[derive(Debug, Clone, Default)]
struct State {
    condition: u32, lineage: u32, repairs: u32, delivered: u64, lineage_hours: u64,
    trace: Vec<u32>, metrics: Totals, fates: FateTotals,
}

pub fn evaluate(bytes: &[u8], source_revision: &str) -> Result<Report, String> {
    let m: Manifest = serde_json::from_slice(bytes).map_err(|e| format!("manifest rejected: {e}"))?;
    validate(&m)?;
    let ev: BTreeMap<&str, &Evidence> = m.evidence.iter().map(|e| (e.id.as_str(), e)).collect();
    let acts: BTreeMap<&str, &Action> = m.actions.iter().map(|a| (a.id.as_str(), a)).collect();
    let digests: BTreeMap<String, bool> = m.evidence.iter()
        .map(|e| (e.id.clone(), sha256_hex(e.payload.as_bytes()) == e.sha256)).collect();
    let all_digests = digests.values().all(|v| *v);
    let mut ss: Vec<&Scenario> = m.scenarios.iter().collect();
    ss.sort_by(|a,b| a.id.cmp(&b.id));
    let mut out = Vec::new();
    let mut plan_count = 0usize;
    for s in ss {
        let mut prod = Vec::new(); let mut oracle = Vec::new();
        for p in &s.plans {
            prod.push(simulate(&m,s,p,&acts,&ev,&digests));
            oracle.push(simulate_oracle(&m,s,p,&acts,&ev,&digests));
        }
        prod.sort_by(|a,b| a.plan_id.cmp(&b.plan_id));
        oracle.sort_by(|a,b| a.plan_id.cmp(&b.plan_id));
        plan_count += prod.len();
        let transition_ok = prod.iter().zip(oracle.iter()).all(|(a,b)| same_outcome(a,b));
        let eligible: Vec<&PlanResult> = prod.iter().filter(|p| p.disposition==Disposition::Eligible).collect();
        let f = frontier(&eligible); let of = frontier_oracle(&eligible);
        let frontier_ok=f==of; let expected_ok=expectations_match(s,&prod,&f);
        out.push(ScenarioResult {
            scenario_id:s.id.clone(), plans:prod,
            eligible_plan_ids:eligible.iter().map(|p|p.plan_id.clone()).collect::<BTreeSet<_>>().into_iter().collect(),
            pareto_frontier:f.into_iter().collect(), oracle_frontier:of.into_iter().collect(),
            transition_oracle_agreement:transition_ok, frontier_oracle_agreement:frontier_ok,
            fixture_expectations_match:expected_ok,
            status:if transition_ok&&frontier_ok&&expected_ok{"PASS"}else{"FAIL"}.into(),
        });
    }
    let overall=if all_digests&&out.iter().all(|s|s.status=="PASS"){"PASS_SYNTHETIC_CONTRACT"}else{"FAIL"};
    Ok(Report {
        tool_version:VERSION.into(),schema_version:m.schema_version,source_revision:source_revision.into(),
        manifest_sha256:sha256_hex(bytes),claim_ceiling:m.claim_ceiling,functional_unit:m.functional_unit,
        asset_mass_g:m.asset_mass_g,horizon_cycles:m.horizon_cycles,evidence_count:m.evidence.len(),
        action_count:m.actions.len(),scenario_count:out.len(),plan_count,all_evidence_digests_valid:all_digests,
        scenarios:out,overall_status:overall.into(),
    })
}
pub fn sha256_hex(bytes:&[u8])->String { let mut s=String::with_capacity(64); for b in Sha256::digest(bytes){write!(&mut s,"{b:02x}").expect("String write cannot fail");} s }
fn validate(m:&Manifest)->Result<(),String>{
    if m.schema_version!="0.1"||m.claim_ceiling!="synthetic_only"{return Err("schema/claim ceiling mismatch".into());}
    if m.scope_id.trim().is_empty()||m.functional_unit.trim().is_empty()||m.asset_mass_g==0||m.service_hours_per_cycle==0||!(1..=12).contains(&m.horizon_cycles){return Err("invalid identity, asset, service unit, or horizon".into());}
    if [m.minimum_condition_bp,m.degradation_per_cycle_bp,m.initial_condition_bp,m.initial_lineage_share_bp].iter().any(|v|*v>10_000){return Err("condition/lineage/degradation exceeds 10000 bp".into());}
    if m.evidence.is_empty()||m.actions.is_empty()||m.scenarios.is_empty(){return Err("evidence/actions/scenarios must be non-empty".into());}
    let mut ids=BTreeSet::new();
    for e in &m.evidence{if e.id.trim().is_empty()||e.payload.is_empty()||!hash_ok(&e.sha256)||!ids.insert(e.id.as_str()){return Err(format!("invalid or duplicate evidence {}",e.id));}}
    let mut action_ids=BTreeSet::new();
    for a in &m.actions{if a.id.trim().is_empty()||!action_ids.insert(a.id.as_str()){return Err(format!("invalid or duplicate action {}",a.id));}if !ids.contains(a.evidence_id.as_str()){return Err(format!("missing action evidence {}",a.id));}}
    let mut scenario_ids=BTreeSet::new();
    for s in &m.scenarios{
        if s.id.trim().is_empty()||!scenario_ids.insert(s.id.as_str())||s.plans.is_empty(){return Err(format!("invalid scenario {}",s.id));}
        let pids:BTreeSet<&str>=s.plans.iter().map(|p|p.id.as_str()).collect();
        if pids.len()!=s.plans.len(){return Err(format!("duplicate plan ID in {}",s.id));}
        let f:BTreeSet<&str>=s.expected_frontier.iter().map(String::as_str).collect();
        if f.len()!=s.expected_frontier.len()||!f.is_subset(&pids){return Err(format!("invalid expected frontier {}",s.id));}
    } Ok(())
}
fn fresh(m:&Manifest)->State{State{condition:m.initial_condition_bp,lineage:m.initial_lineage_share_bp,..State::default()}}
fn finish(s:&Scenario,p:&Plan,d:Disposition,reason:String,x:State,m:&Manifest)->PlanResult{
    PlanResult{scenario_id:s.id.clone(),plan_id:p.id.clone(),complete_service_horizon:d==Disposition::Eligible&&x.trace.len()==m.horizon_cycles,disposition:d,reason,delivered_service_hours:x.delivered,lineage_weighted_service_hours:x.lineage_hours,condition_after_cycle_bp:x.trace,final_condition_bp:x.condition,final_lineage_share_bp:x.lineage,repairs_used:x.repairs,metrics:x.metrics,fates:x.fates}
}
fn status_block(a:&Action,e:&Evidence,m:&Manifest,hash:bool)->Option<(Disposition,String)>{
    if !hash{return Some((Disposition::InvalidDigest,"evidence digest mismatch".into()));}
    if matches!(&e.status,EvidenceStatus::Stale|EvidenceStatus::Revoked){return Some((Disposition::RejectedEvidence,"evidence stale/revoked".into()));}
    if e.claim_id!=CLAIM||e.scope_id!=m.scope_id{return Some((Disposition::OutOfScope,"evidence claim/scope mismatch".into()));}
    if e.coverage==Coverage::Partial{return Some((Disposition::BlockedIncompleteCoverage,"partial coverage".into()));}
    if e.status==EvidenceStatus::Unknown||e.coverage==Coverage::Unknown{return Some((Disposition::BlockedUnknown,"unknown evidence".into()));}
    if a.safety==Gate::Fail||a.function==Gate::Fail||a.inspection==Gate::Fail{return Some((Disposition::RejectedHardGate,"required gate failed".into()));}
    if a.safety==Gate::Unknown||a.function==Gate::Unknown||a.inspection==Gate::Unknown{return Some((Disposition::BlockedUnknown,"required gate unknown".into()));}
    None
}
fn mass_ok(a:&Action,m:&Manifest)->bool{
    let f=&a.fates;let sum=u128::from(f.reused_component_g)+u128::from(f.recycled_material_g)+u128::from(f.energy_recovery_g)+u128::from(f.residual_g);
    let replace=matches!(&a.kind,Kind::ReplaceRecycle|Kind::ReplaceEnergyRecovery|Kind::ReplaceMixed);
    sum==u128::from(a.retired_mass_g)&&a.metrics.residual_waste_g==f.residual_g&&if replace{a.retired_mass_g==m.asset_mass_g}else{a.retired_mass_g<=m.asset_mass_g}
}
fn effects_ok(a:&Action)->bool{
    if a.recovery_bp>10_000||a.lineage_after_bp.is_some_and(|v|v>10_000){return false;}
    match &a.kind{Kind::Continue=>a.recovery_bp==0&&a.lineage_after_bp.is_none(),Kind::Repair=>a.recovery_bp>0&&a.lineage_after_bp.is_none(),Kind::Remanufacture=>a.recovery_bp==0&&a.lineage_after_bp.is_some(),Kind::ReplaceRecycle|Kind::ReplaceEnergyRecovery|Kind::ReplaceMixed=>a.recovery_bp==0&&a.lineage_after_bp==Some(0)}
}
fn add_metrics(x:&mut Totals,m:&Metrics)->bool{
    macro_rules! add{($f:ident)=>{if let Some(v)=x.$f.checked_add(m.$f){x.$f=v}else{return false}}}
    add!(cost_cents);add!(virgin_material_g);add!(energy_wh);add!(ghg_co2e_g);add!(water_ml);add!(residual_waste_g);true
}
fn add_fates(x:&mut FateTotals,a:&Action)->bool{
    macro_rules! add{($f:ident,$v:expr)=>{if let Some(v)=x.$f.checked_add($v){x.$f=v}else{return false}}}
    add!(retired_mass_g,a.retired_mass_g);add!(reused_component_g,a.fates.reused_component_g);add!(recycled_material_g,a.fates.recycled_material_g);add!(energy_recovery_g,a.fates.energy_recovery_g);add!(residual_g,a.fates.residual_g);true
}
fn simulate(m:&Manifest,s:&Scenario,p:&Plan,acts:&BTreeMap<&str,&Action>,ev:&BTreeMap<&str,&Evidence>,hashes:&BTreeMap<String,bool>)->PlanResult{
    let mut x=fresh(m);if p.steps.len()!=m.horizon_cycles{return finish(s,p,Disposition::InvalidPlan,"plan length differs from horizon".into(),x,m);}
    for (i,id) in p.steps.iter().enumerate(){
        let Some(a)=acts.get(id.as_str()).copied() else{return finish(s,p,Disposition::InvalidPlan,format!("unknown action {id}"),x,m);};
        let Some(e)=ev.get(a.evidence_id.as_str()).copied() else{return finish(s,p,Disposition::InvalidPlan,"missing evidence ref".into(),x,m);};
        if let Some((d,r))=status_block(a,e,m,hashes.get(&e.id).copied().unwrap_or(false)){return finish(s,p,d,r,x,m);}
        if !mass_ok(a,m){return finish(s,p,Disposition::RejectedMassBalance,format!("cycle {} fates do not conserve mass",i+1),x,m);}
        if !effects_ok(a){return finish(s,p,Disposition::InvalidMetrics,format!("cycle {} invalid state effect",i+1),x,m);}
        match &a.kind{
            Kind::Continue=>{},
            Kind::Repair=>{if x.repairs>=m.maximum_repairs{return finish(s,p,Disposition::RejectedRepairLimit,format!("cycle {} repair budget exhausted",i+1),x,m);}x.repairs+=1;x.condition=x.condition.saturating_add(a.recovery_bp).min(10_000);},
            Kind::Remanufacture=>{x.condition=10_000;x.lineage=a.lineage_after_bp.unwrap_or(0);x.repairs=0;},
            Kind::ReplaceRecycle|Kind::ReplaceEnergyRecovery|Kind::ReplaceMixed=>{x.condition=10_000;x.lineage=0;x.repairs=0;},
        }
        if x.condition<m.minimum_condition_bp{return finish(s,p,Disposition::RejectedCondition,format!("cycle {} condition below minimum",i+1),x,m);}
        if !add_metrics(&mut x.metrics,&a.metrics)||!add_fates(&mut x.fates,a){return finish(s,p,Disposition::InvalidMetrics,"cumulative metric overflow".into(),x,m);}
        let Some(weighted)=m.service_hours_per_cycle.checked_mul(u64::from(x.lineage)) else{return finish(s,p,Disposition::InvalidMetrics,"lineage hour overflow".into(),x,m);};
        let Some(delivered)=x.delivered.checked_add(m.service_hours_per_cycle) else{return finish(s,p,Disposition::InvalidMetrics,"service hour overflow".into(),x,m);};
        let Some(lineage)=x.lineage_hours.checked_add(weighted/10_000) else{return finish(s,p,Disposition::InvalidMetrics,"lineage hour overflow".into(),x,m);};
        x.delivered=delivered;x.lineage_hours=lineage;x.condition=x.condition.saturating_sub(m.degradation_per_cycle_bp);x.trace.push(x.condition);
    } finish(s,p,Disposition::Eligible,"complete horizon delivered".into(),x,m)
}
fn simulate_oracle(m:&Manifest,s:&Scenario,p:&Plan,acts:&BTreeMap<&str,&Action>,ev:&BTreeMap<&str,&Evidence>,hashes:&BTreeMap<String,bool>)->PlanResult{
    let mut x=fresh(m);if p.steps.len()!=m.horizon_cycles{return finish(s,p,Disposition::InvalidPlan,"oracle: incorrect horizon length".into(),x,m);}
    for (i,id) in p.steps.iter().enumerate(){
        let a=match acts.get(id.as_str()){Some(a)=>*a,None=>return finish(s,p,Disposition::InvalidPlan,"oracle: unregistered action".into(),x,m)};
        let e=match ev.get(a.evidence_id.as_str()){Some(e)=>*e,None=>return finish(s,p,Disposition::InvalidPlan,"oracle: unregistered evidence".into(),x,m)};
        if !hashes.get(&e.id).copied().unwrap_or(false){return finish(s,p,Disposition::InvalidDigest,"oracle: source hash mismatch".into(),x,m);}
        if matches!(&e.status,EvidenceStatus::Stale|EvidenceStatus::Revoked){return finish(s,p,Disposition::RejectedEvidence,"oracle: source not admissible".into(),x,m);}
        if e.claim_id!=CLAIM||e.scope_id!=m.scope_id{return finish(s,p,Disposition::OutOfScope,"oracle: claim/scope differ".into(),x,m);}
        if matches!(&e.coverage,Coverage::Partial){return finish(s,p,Disposition::BlockedIncompleteCoverage,"oracle: partial coverage".into(),x,m);}
        if matches!(&e.status,EvidenceStatus::Unknown)||matches!(&e.coverage,Coverage::Unknown){return finish(s,p,Disposition::BlockedUnknown,"oracle: source unknown".into(),x,m);}
        if [&a.safety,&a.function,&a.inspection].iter().any(|g|**g==Gate::Fail){return finish(s,p,Disposition::RejectedHardGate,"oracle: gate failed".into(),x,m);}
        if [&a.safety,&a.function,&a.inspection].iter().any(|g|**g==Gate::Unknown){return finish(s,p,Disposition::BlockedUnknown,"oracle: gate unknown".into(),x,m);}
        let f=&a.fates;let sum=u128::from(f.reused_component_g)+u128::from(f.recycled_material_g)+u128::from(f.energy_recovery_g)+u128::from(f.residual_g);
        let replacing=matches!(&a.kind,Kind::ReplaceRecycle|Kind::ReplaceEnergyRecovery|Kind::ReplaceMixed);
        if sum!=u128::from(a.retired_mass_g)||a.metrics.residual_waste_g!=f.residual_g||(replacing&&a.retired_mass_g!=m.asset_mass_g)||(!replacing&&a.retired_mass_g>m.asset_mass_g){return finish(s,p,Disposition::RejectedMassBalance,format!("oracle: cycle {} fate mismatch",i+1),x,m);}
        if !effects_ok(a){return finish(s,p,Disposition::InvalidMetrics,"oracle: malformed state effect".into(),x,m);}
        match &a.kind{
            Kind::Continue=>{},
            Kind::Repair=>{if m.maximum_repairs<=x.repairs{return finish(s,p,Disposition::RejectedRepairLimit,"oracle: no repair allowance".into(),x,m);}x.repairs+=1;x.condition=std::cmp::min(10_000,x.condition.saturating_add(a.recovery_bp));},
            Kind::Remanufacture=>{x.condition=10_000;x.lineage=a.lineage_after_bp.unwrap_or(0);x.repairs=0;},
            Kind::ReplaceRecycle|Kind::ReplaceEnergyRecovery|Kind::ReplaceMixed=>{x.condition=10_000;x.lineage=0;x.repairs=0;},
        }
        if x.condition<m.minimum_condition_bp{return finish(s,p,Disposition::RejectedCondition,"oracle: below condition floor".into(),x,m);}
        macro_rules! add{($f:ident,$v:expr)=>{match x.metrics.$f.checked_add($v){Some(n)=>x.metrics.$f=n,None=>return finish(s,p,Disposition::InvalidMetrics,"oracle: metric overflow".into(),x,m)}}}
        add!(cost_cents,a.metrics.cost_cents);add!(virgin_material_g,a.metrics.virgin_material_g);add!(energy_wh,a.metrics.energy_wh);add!(ghg_co2e_g,a.metrics.ghg_co2e_g);add!(water_ml,a.metrics.water_ml);add!(residual_waste_g,a.metrics.residual_waste_g);
        macro_rules! fate{($f:ident,$v:expr)=>{match x.fates.$f.checked_add($v){Some(n)=>x.fates.$f=n,None=>return finish(s,p,Disposition::InvalidMetrics,"oracle: fate overflow".into(),x,m)}}}
        fate!(retired_mass_g,a.retired_mass_g);fate!(reused_component_g,a.fates.reused_component_g);fate!(recycled_material_g,a.fates.recycled_material_g);fate!(energy_recovery_g,a.fates.energy_recovery_g);fate!(residual_g,a.fates.residual_g);
        let weight=match m.service_hours_per_cycle.checked_mul(u64::from(x.lineage)){Some(n)=>n/10_000,None=>return finish(s,p,Disposition::InvalidMetrics,"oracle: lineage overflow".into(),x,m)};
        x.delivered=match x.delivered.checked_add(m.service_hours_per_cycle){Some(n)=>n,None=>return finish(s,p,Disposition::InvalidMetrics,"oracle: service overflow".into(),x,m)};
        x.lineage_hours=match x.lineage_hours.checked_add(weight){Some(n)=>n,None=>return finish(s,p,Disposition::InvalidMetrics,"oracle: lineage overflow".into(),x,m)};
        x.condition=if x.condition>m.degradation_per_cycle_bp{x.condition-m.degradation_per_cycle_bp}else{0};x.trace.push(x.condition);
    }finish(s,p,Disposition::Eligible,"oracle: horizon complete".into(),x,m)
}
fn same_outcome(a:&PlanResult,b:&PlanResult)->bool{
    a.scenario_id==b.scenario_id&&a.plan_id==b.plan_id&&a.disposition==b.disposition&&a.complete_service_horizon==b.complete_service_horizon
    &&a.delivered_service_hours==b.delivered_service_hours&&a.lineage_weighted_service_hours==b.lineage_weighted_service_hours
    &&a.condition_after_cycle_bp==b.condition_after_cycle_bp&&a.final_condition_bp==b.final_condition_bp&&a.final_lineage_share_bp==b.final_lineage_share_bp
    &&a.repairs_used==b.repairs_used&&a.metrics==b.metrics&&a.fates==b.fates
}
fn dominates(a:&PlanResult,b:&PlanResult)->bool{
    let no=a.metrics.cost_cents<=b.metrics.cost_cents&&a.metrics.virgin_material_g<=b.metrics.virgin_material_g&&a.metrics.energy_wh<=b.metrics.energy_wh
    &&a.metrics.ghg_co2e_g<=b.metrics.ghg_co2e_g&&a.metrics.water_ml<=b.metrics.water_ml&&a.metrics.residual_waste_g<=b.metrics.residual_waste_g
    &&a.lineage_weighted_service_hours>=b.lineage_weighted_service_hours;
    no&&(a.metrics.cost_cents<b.metrics.cost_cents||a.metrics.virgin_material_g<b.metrics.virgin_material_g||a.metrics.energy_wh<b.metrics.energy_wh
    ||a.metrics.ghg_co2e_g<b.metrics.ghg_co2e_g||a.metrics.water_ml<b.metrics.water_ml||a.metrics.residual_waste_g<b.metrics.residual_waste_g
    ||a.lineage_weighted_service_hours>b.lineage_weighted_service_hours)
}
fn frontier(ps:&[&PlanResult])->BTreeSet<String>{ps.iter().filter(|p|!ps.iter().any(|q|q.plan_id!=p.plan_id&&dominates(q,p))).map(|p|p.plan_id.clone()).collect()}
fn frontier_oracle(ps:&[&PlanResult])->BTreeSet<String>{
    let v=|p:&PlanResult|[i128::from(p.metrics.cost_cents),i128::from(p.metrics.virgin_material_g),i128::from(p.metrics.energy_wh),i128::from(p.metrics.ghg_co2e_g),i128::from(p.metrics.water_ml),i128::from(p.metrics.residual_waste_g),-i128::from(p.lineage_weighted_service_hours)];
    let mut out=BTreeSet::new();for p in ps{let pv=v(p);let mut loss=false;for q in ps{if p.plan_id==q.plan_id{continue;}let qv=v(q);if (0..7).all(|i|qv[i]<=pv[i])&&(0..7).any(|i|qv[i]<pv[i]){loss=true;break;}}if !loss{out.insert(p.plan_id.clone());}}out
}
fn expectations_match(s:&Scenario,rs:&[PlanResult],f:&BTreeSet<String>)->bool{
    if rs.len()!=s.plans.len(){return false;}for r in rs{let Some(p)=s.plans.iter().find(|p|p.id==r.plan_id)else{return false;};if r.disposition!=p.expected.disposition||r.delivered_service_hours!=p.expected.delivered_service_hours||r.lineage_weighted_service_hours!=p.expected.lineage_weighted_service_hours||r.condition_after_cycle_bp!=p.expected.condition_after_cycle_bp{return false;}}
    *f==s.expected_frontier.iter().cloned().collect()
}
fn hash_ok(s:&str)->bool{s.len()==64&&s.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b))}

#[cfg(test)]
mod tests{
 use super::*;const FIXTURE:&str=include_str!("../fixtures/lifecycle-horizon-v0-1.json");
 fn manifest()->Manifest{serde_json::from_str(FIXTURE).unwrap()}fn report()->Report{evaluate(FIXTURE.as_bytes(),"test-revision").unwrap()}
 fn plan<'a>(r:&'a Report,s:&str,p:&str)->&'a PlanResult{r.scenarios.iter().find(|x|x.scenario_id==s).unwrap().plans.iter().find(|x|x.plan_id==p).unwrap()}
 #[test]fn frozen_plans_match_transition_and_frontier_oracles(){let r=report();assert_eq!(r.overall_status,"PASS_SYNTHETIC_CONTRACT");for s in &r.scenarios{assert_eq!(s.status,"PASS");assert!(s.transition_oracle_agreement);assert!(s.frontier_oracle_agreement);assert!(s.fixture_expectations_match);}}
 #[test]fn no_maintenance_cannot_claim_full_horizon(){let p=plan(&report(),"baseline-horizon","no-maintenance");assert_eq!(p.disposition,Disposition::RejectedCondition);assert_eq!(p.delivered_service_hours,20_000);assert!(!p.complete_service_horizon);}
 #[test]fn remanufacture_restores_condition_but_reduces_lineage(){let p=plan(&report(),"baseline-horizon","remanufacture-cycle-2");assert_eq!(p.delivered_service_hours,30_000);assert_eq!(p.lineage_weighted_service_hours,20_000);assert_eq!(p.condition_after_cycle_bp,vec![7500,7500,5000]);}
 #[test]fn repair_preserves_more_lineage_service_than_remanufacture(){let r=report();assert_eq!(plan(&r,"baseline-horizon","repair-cycle-2").lineage_weighted_service_hours,30_000);assert_eq!(plan(&r,"baseline-horizon","remanufacture-cycle-2").lineage_weighted_service_hours,20_000);}
 #[test]fn evidence_and_gates_fail_closed(){let r=report();assert_eq!(plan(&r,"evidence-and-gates","unknown-evidence").disposition,Disposition::BlockedUnknown);assert_eq!(plan(&r,"evidence-and-gates","partial-evidence").disposition,Disposition::BlockedIncompleteCoverage);assert_eq!(plan(&r,"evidence-and-gates","revoked-evidence").disposition,Disposition::RejectedEvidence);assert_eq!(plan(&r,"evidence-and-gates","failed-inspection").disposition,Disposition::RejectedHardGate);assert_eq!(plan(&r,"evidence-and-gates","scope-mismatch").disposition,Disposition::OutOfScope);}
 #[test]fn repair_budget_and_bad_fates_are_rejected(){let r=report();assert_eq!(plan(&r,"repair-limit-and-fates","repair-too-many-times").disposition,Disposition::RejectedRepairLimit);assert_eq!(plan(&r,"repair-limit-and-fates","bad-mass-fates").disposition,Disposition::RejectedMassBalance);}
 #[test]fn energy_recovery_is_not_recycling(){let r=report();let s=r.scenarios.iter().find(|s|s.scenario_id=="replacement-fates").unwrap();let a=s.plans.iter().find(|p|p.plan_id=="replace-recycle").unwrap();let b=s.plans.iter().find(|p|p.plan_id=="replace-energy-recovery").unwrap();assert_eq!(a.fates.recycled_material_g,600);assert_eq!(a.fates.energy_recovery_g,0);assert_eq!(b.fates.recycled_material_g,100);assert_eq!(b.fates.energy_recovery_g,600);assert_eq!(b.fates.retired_mass_g,1000);}
 #[test]fn wrong_length_and_unknown_action_are_invalid_plans(){assert_eq!(plan(&report(),"repair-limit-and-fates","short-plan").disposition,Disposition::InvalidPlan);assert_eq!(plan(&report(),"repair-limit-and-fates","unknown-action").disposition,Disposition::InvalidPlan);}
 #[test]fn mutated_digest_blocks_plan(){let mut m=manifest();m.evidence.iter_mut().find(|e|e.id=="ev-repair").unwrap().sha256="0".repeat(64);let b=serde_json::to_vec(&m).unwrap();let r=evaluate(&b,"test").unwrap();assert_eq!(r.overall_status,"FAIL");assert_eq!(plan(&r,"baseline-horizon","repair-cycle-2").disposition,Disposition::InvalidDigest);}
 #[test]fn undeclared_fields_are_rejected(){let bad=FIXTURE.replace(r#""claim_ceiling": "synthetic_only""#,r#""claim_ceiling": "synthetic_only", "auto_approve": true"#);assert!(evaluate(bad.as_bytes(),"test").is_err());}
 #[test]fn sha256_known_answer(){assert_eq!(sha256_hex(b"abc"),"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");}
}

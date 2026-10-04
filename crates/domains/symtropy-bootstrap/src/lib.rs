use std::collections::{BTreeMap,BTreeSet};

#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
pub enum DependencyClass{LocalClosed,LocalOpen,ImportedConsumable,ImportedDurable,ReplaceableBySubstitution,Unknown}
impl DependencyClass{pub const fn closed(self)->bool{matches!(self,Self::LocalClosed|Self::ReplaceableBySubstitution)}}

#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Dependency{pub id:String,pub class:DependencyClass}
impl Dependency{pub fn new(id:impl Into<String>,class:DependencyClass)->Self{Self{id:id.into(),class}}}

#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Capability{pub id:String,pub weight:u64,pub deps:Vec<String>}
impl Capability{pub fn new(id:impl Into<String>,weight:u64,deps:impl IntoIterator<Item=impl Into<String>>)->Self{Self{id:id.into(),weight,deps:deps.into_iter().map(Into::into).collect()}}}

#[derive(Debug,Clone,PartialEq,Eq)]
pub struct CapabilityAssessment{pub id:String,pub closed:bool,pub unresolved:Vec<String>,pub cycle:bool}

#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ClosureReport{pub assessments:Vec<CapabilityAssessment>,pub critical_closed:u64,pub critical_total:u64,pub critical_ppm:u64,pub mass_ppm:u64}
impl ClosureReport{pub const fn fully_closed(&self)->bool{self.critical_closed==self.critical_total}}

#[derive(Debug,Default,Clone)]
pub struct DependencyGraph{caps:BTreeMap<String,Capability>,deps:BTreeMap<String,Dependency>}
impl DependencyGraph{
pub fn new(c:impl IntoIterator<Item=Capability>,d:impl IntoIterator<Item=Dependency>)->Self{Self{caps:c.into_iter().map(|x|(x.id.clone(),x)).collect(),deps:d.into_iter().map(|x|(x.id.clone(),x)).collect()}}
pub fn evaluate(&self,local:u64,imported:u64)->ClosureReport{let mut m=BTreeMap::new();let mut a=Vec::new();for id in self.caps.keys(){let(o,mut u,c)=self.resolve(id,&mut m,&mut Vec::new());u.sort();u.dedup();a.push(CapabilityAssessment{id:id.clone(),closed:o,unresolved:u,cycle:c})}let total=self.caps.values().map(|x|x.weight).sum();let closed=a.iter().filter(|x|x.closed).filter_map(|x|self.caps.get(&x.id)).map(|x|x.weight).sum();ClosureReport{assessments:a,critical_closed:closed,critical_total:total,critical_ppm:ratio_ppm(closed,total),mass_ppm:ratio_ppm(local,local.saturating_add(imported))}}
fn resolve(&self,id:&str,m:&mut BTreeMap<String,bool>,s:&mut Vec<String>)->(bool,Vec<String>,bool){if let Some(v)=m.get(id){return(*v,vec![],false)}if s.iter().any(|x|x==id){return(false,vec![id.into()],true)}if let Some(d)=self.deps.get(id){let o=d.class.closed();m.insert(id.into(),o);return(o,if o{vec![]}else{vec![id.into()]},false)}let Some(c)=self.caps.get(id)else{return(false,vec![id.into()],false)};s.push(id.into());let(mut o,u,mut cy)=(true,Vec::new(),false);for d in &c.deps{let(q,mut miss,z)=if self.caps.contains_key(d){self.resolve(d,m,s)}else if let Some(x)=self.deps.get(d){(x.class.closed(),if x.class.closed(){vec![]}else{vec![d.clone()]},false)}else{(false,vec![d.clone()],false)};o&=q;u.append(&mut miss);cy|=z}s.pop();m.insert(id.into(),o);(o,u,cy)}
}

#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
pub enum ClosureStage{Seed,Repair,Feedstock,Structural,Machine,Control,Factory,Ecological,Expansion}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct StageRequirement{pub stage:ClosureStage,pub capabilities:Vec<String>}
impl StageRequirement{pub fn new(stage:ClosureStage,c:impl IntoIterator<Item=impl Into<String>>)->Self{Self{stage,capabilities:c.into_iter().map(Into::into).collect()}}}
pub fn highest_closed_stage(r:&ClosureReport,q:&[StageRequirement])->Option<ClosureStage>{let c=r.assessments.iter().filter(|x|x.closed).map(|x|x.id.as_str()).collect::<BTreeSet<_>>();q.iter().filter(|x|x.capabilities.iter().all(|i|c.contains(i.as_str()))).map(|x|x.stage).max()}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct BootstrapEfficiency{pub gain:u64,pub imported_mass_g:u64}
impl BootstrapEfficiency{pub const fn new(gain:u64,mass:u64)->Self{Self{gain,imported_mass_g:mass}}pub fn better_than(self,o:Self)->bool{match(self.imported_mass_g,o.imported_mass_g){(0,0)=>self.gain>o.gain,(0,_)=>self.gain>0,(_,0)=>false,_=>(self.gain as u128)*(o.imported_mass_g as u128)>(o.gain as u128)*(self.imported_mass_g as u128)}}}

#[derive(Debug,Clone,PartialEq,Eq)]
pub enum RecoveryOutcome{Immediate,RecoveredAfter{ticks:u64},Unrecoverable}
pub const fn recovery_horizon(x:&RecoveryOutcome)->Option<u64>{match x{RecoveryOutcome::Immediate=>Some(0),RecoveryOutcome::RecoveredAfter{ticks}=>Some(*ticks),RecoveryOutcome::Unrecoverable=>None}}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum InventoryEventKind{Produced,Consumed,Recycled,TransferredIn,TransferredOut}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct InventoryEvent{pub sequence:u64,pub batch_id:String,pub mass_g:u64,pub kind:InventoryEventKind}
impl InventoryEvent{pub fn new(sequence:u64,batch_id:impl Into<String>,mass_g:u64,kind:InventoryEventKind)->Self{Self{sequence,batch_id:batch_id.into(),mass_g,kind}}}
pub fn replay_inventory(i:&BTreeMap<String,u64>,e:&[InventoryEvent])->Result<BTreeMap<String,u64>,String>{let mut e=e.to_vec();e.sort_by_key(|x|x.sequence);if e.windows(2).any(|w|w[0].sequence==w[1].sequence){return Err("duplicate sequence".into())}let mut o=i.clone();for x in e{let b=o.entry(x.batch_id).or_insert(0);match x.kind{InventoryEventKind::Produced|InventoryEventKind::Recycled|InventoryEventKind::TransferredIn=>*b=b.checked_add(x.mass_g).ok_or("overflow")?,InventoryEventKind::Consumed|InventoryEventKind::TransferredOut=>{if *b<x.mass_g{return Err("underflow".into())}*b-=x.mass_g}}}Ok(o)}
pub fn verify_inventory_conservation(i:&BTreeMap<String,u64>,e:&[InventoryEvent],f:&BTreeMap<String,u64>)->Result<(),String>{if replay_inventory(i,e)?==*f{Ok(())}else{Err("observed inventory differs from causal replay".into())}}
pub const fn ratio_ppm(n:u64,d:u64)->u64{if d==0{0}else{(((n as u128)*1_000_000)/(d as u128)).min(1_000_000)as u64}}

#[cfg(test)]mod tests{use super::*;fn report()->ClosureReport{DependencyGraph::new([Capability::new("mine",30,["rock"]),Capability::new("refine",40,["mine","chem"]),Capability::new("ctrl",30,["electronics"])],[Dependency::new("rock",DependencyClass::LocalClosed),Dependency::new("chem",DependencyClass::LocalClosed),Dependency::new("electronics",DependencyClass::ImportedDurable)]).evaluate(9000,1000)}
#[test]fn bottleneck_is_visible(){let r=report();assert_eq!(r.mass_ppm,900000);assert_eq!(r.critical_ppm,700000);assert!(!r.fully_closed())}
#[test]fn unknown_fails_closed(){let r=DependencyGraph::new([Capability::new("r",100,["mystery"])],[]).evaluate(1,0);assert!(!r.assessments[0].closed)}
#[test]fn recursive_chain_closes(){let r=DependencyGraph::new([Capability::new("a",50,["steel"]),Capability::new("b",50,["a"])],[Dependency::new("steel",DependencyClass::LocalClosed)]).evaluate(1,0);assert!(r.fully_closed())}
#[test]fn cycles_fail_closed(){let r=DependencyGraph::new([Capability::new("a",50,["b"]),Capability::new("b",50,["a"])],[]).evaluate(1,0);assert!(r.assessments.iter().any(|x|x.cycle));assert!(!r.fully_closed())}
#[test]fn stage_and_efficiency_are_exact(){let r=DependencyGraph::new([Capability::new("repair",10,["tool"]),Capability::new("structure",10,["repair"])],[Dependency::new("tool",DependencyClass::LocalClosed)]).evaluate(1,0);let q=[StageRequirement::new(ClosureStage::Repair,["repair"]),StageRequirement::new(ClosureStage::Structural,["repair","structure"])];assert_eq!(highest_closed_stage(&r,&q),Some(ClosureStage::Structural));assert!(BootstrapEfficiency::new(3,2).better_than(BootstrapEfficiency::new(4,3)))}
#[test]fn recovery_is_explicit(){assert_eq!(recovery_horizon(&RecoveryOutcome::RecoveredAfter{ticks:7}),Some(7));assert_eq!(recovery_horizon(&RecoveryOutcome::Unrecoverable),None)}
#[test]fn inventory_is_causal_and_order_independent(){let i=BTreeMap::from([("steel".into(),100)]);let e=[InventoryEvent::new(1,"steel",25,InventoryEventKind::Produced),InventoryEvent::new(2,"steel",60,InventoryEventKind::Consumed),InventoryEvent::new(3,"steel",10,InventoryEventKind::Recycled)];let f=BTreeMap::from([("steel".into(),75)]);verify_inventory_conservation(&i,&e,&f).unwrap();assert_eq!(replay_inventory(&i,&[e[2].clone(),e[0].clone(),e[1].clone()]).unwrap(),f)}
#[test]fn inventory_rejects_bad_history(){let i=BTreeMap::from([("x".into(),1)]);assert!(replay_inventory(&i,&[InventoryEvent::new(1,"x",2,InventoryEventKind::Consumed)]).is_err());let d=[InventoryEvent::new(1,"a",1,InventoryEventKind::Produced),InventoryEvent::new(1,"b",1,InventoryEventKind::Produced)];assert!(replay_inventory(&BTreeMap::new(),&d).is_err())}}

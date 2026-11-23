import fs from "node:fs";
const schemaId="luminous.orbital-loss-model-universe-adequacy.corpus.v0.1";
const ceiling="synthetic-loss-model-universe-adequacy-only";
const grammar={
 orbital_vehicle:[
  {family:"propellant_consumption",id:"propellant-v1",tag:null},
  {family:"thermal_stress",id:"thermal-v1",tag:"thermal"},
  {family:"propulsion_degradation",id:"propulsion-v1",tag:"propulsion"},
  {family:"avionics_reliability",id:"avionics-v1",tag:null}
 ],
 powered_flight:[
  {family:"propellant_consumption",id:"propellant-v1",tag:null},
  {family:"thermal_stress",id:"thermal-v1",tag:"thermal"},
  {family:"avionics_reliability",id:"avionics-v1",tag:null}
 ]
};
const cmp=(a,b)=>a<b?-1:a>b?1:0;
const validId=s=>typeof s==="string"&&s.trim().length>0&&[...s].every(ch=>{const n=ch.codePointAt(0);return (n>=33&&n<=126)||n===32;});
const validSha=s=>typeof s==="string"&&/^[0-9a-f]{64}$/.test(s);
const pairId=(w,m)=>w+"::"+m;
const mkFinding=(code,model,pair,detail)=>({code,model_id:model??null,pair_id:pair??null,detail});
function invalid(c,detail){return {case_id:c.case_id,disposition:"INVALID_CASE",expected_disposition:c.expected.disposition,
 missing_model_ids:[],missing_pair_ids:[],unexpected_model_ids:[],unexpected_pair_ids:[],unwitnessed_exclusion_count:0,
 findings:[mkFinding("INVALID_CASE",null,null,detail)],producer_model_count:c.producer_models.length,
 declared_model_count:c.declared_model_ids.length,expected_model_count:0,expected_pair_count:0,actual_pair_count:c.joint_pairs.length,
 expected_result_matched:c.expected.disposition==="INVALID_CASE"};}
function evalCase(c){
 const reqs=grammar[c.mission_class];
 if(!reqs)return {case_id:c.case_id,disposition:"MODEL_FAMILY_INADEQUATE",expected_disposition:c.expected.disposition,
  missing_model_ids:[],missing_pair_ids:[],unexpected_model_ids:[],unexpected_pair_ids:[],unwitnessed_exclusion_count:0,
  findings:[mkFinding("MODEL_FAMILY_INADEQUATE",null,null,"mission class has no independent model-family grammar")],
  producer_model_count:c.producer_models.length,declared_model_count:c.declared_model_ids.length,
  expected_model_count:0,expected_pair_count:0,actual_pair_count:c.joint_pairs.length,
  expected_result_matched:c.expected.disposition==="MODEL_FAMILY_INADEQUATE"&&
   !c.expected.missing_model_ids.length&&!c.expected.missing_pair_ids.length&&!c.expected.unexpected_model_ids.length&&!c.expected.unexpected_pair_ids.length};
 if(!validId(c.case_id)||!validSha(c.scope_sha256)||!c.worlds.length||c.worlds.length>4096||
   c.producer_models.length>1024||c.declared_model_ids.length>1024||c.applicability.length>16384||c.joint_pairs.length>16384)
  return invalid(c,"invalid scope, identity or finite bounds");
 const worlds=new Map();
 for(const w of c.worlds){
  if(!validId(w.world_id)||!validSha(w.scope_sha256)||w.scope_sha256!==c.scope_sha256||
   w.profile_tags.some(t=>!validId(t))||worlds.has(w.world_id))return invalid(c,"malformed, duplicate or out-of-scope physical world");
  worlds.set(w.world_id,w);
 }
 const models=new Map(),receiptOwners=new Map(),findings=[];
 for(const m of c.producer_models){
  if(!validId(m.model_id)||!validId(m.family)||!validId(m.version)||!validSha(m.scope_sha256)||
   !["CURRENT","STALE","OUT_OF_PROFILE","INDETERMINATE"].includes(m.state)||models.has(m.model_id))
   return invalid(c,"malformed or duplicate producer model");
  models.set(m.model_id,m);
  if(m.scope_sha256!==c.scope_sha256)findings.push(mkFinding("MODEL_SCOPE_MISMATCH",m.model_id,null,"producer model scope differs from the campaign scope"));
  if(m.source_receipt_id){
   const prior=receiptOwners.get(m.source_receipt_id);
   if(prior&&prior!==m.model_id){
    const [first,second]=[prior,m.model_id].sort(cmp);
    findings.push(mkFinding("MODEL_RECEIPT_ALIAS",first,null,
      "source receipt "+m.source_receipt_id+" is shared by models "+first+" and "+second));
    receiptOwners.set(m.source_receipt_id,first);
   } else if(!prior) receiptOwners.set(m.source_receipt_id,m.model_id);
  }
 }
 const reqById=new Map(reqs.map(r=>[r.id,r]));
 const expectedModels=new Set(reqs.map(r=>r.id));
 const producerIds=new Set(models.keys()),declaredIds=new Set(c.declared_model_ids);
 if(declaredIds.size!==c.declared_model_ids.length||[...declaredIds].some(id=>!validId(id)))
  return invalid(c,"duplicate or malformed declared model ID");
 const missingModels=new Set(),unexpectedModels=new Set();
 for(const id of expectedModels){
  const p=producerIds.has(id),d=declaredIds.has(id);
  if(!p&&!d){missingModels.add(id);findings.push(mkFinding("COMMON_MODE_MODEL_OMISSION",id,null,"independent grammar requires a model omitted by both source inventories"));}
  else if(!p||!d){missingModels.add(id);findings.push(mkFinding("MISSING_LOSS_MODEL",id,null,"required model missing from producer or declaration inventory"));}
 }
 for(const id of new Set([...producerIds,...declaredIds])){
  if(!expectedModels.has(id)){unexpectedModels.add(id);findings.push(mkFinding("UNEXPECTED_MODEL",id,null,"model is outside the independently registered family grammar"));}
 }
 for(const [id,m] of models){
  const code=m.state==="STALE"?"MODEL_STALE":m.state==="OUT_OF_PROFILE"?"MODEL_OUT_OF_PROFILE":m.state==="INDETERMINATE"?"MODEL_SOURCE_UNAVAILABLE":null;
  if(code)findings.push(mkFinding(code,id,null,"model state prevents closed model-family evidence"));
  if(!m.source_receipt_id||!validId(m.source_receipt_id))findings.push(mkFinding("MODEL_SOURCE_UNAVAILABLE",id,null,"model lacks a source receipt identity"));
 }
 const expectedPairs=new Set();
 for(const w of c.worlds)for(const r of reqs)if(r.tag===null||w.profile_tags.includes(r.tag))expectedPairs.add(pairId(w.world_id,r.id));
 const applicability=new Map();let unwitnessed=0;
 for(const row of c.applicability){
  if(!worlds.has(row.world_id)||!validId(row.model_id))return invalid(c,"unknown or malformed applicability identity");
  const key=row.world_id+"\u0000"+row.model_id;
  if(applicability.has(key))return invalid(c,"duplicate applicability identity");
  applicability.set(key,row);
  if(!reqById.has(row.model_id))findings.push(mkFinding("UNEXPECTED_APPLICABILITY",row.model_id,pairId(row.world_id,row.model_id),"applicability references an unknown model"));
 }
 for(const w of c.worlds)for(const req of reqs){
  if(!producerIds.has(req.id)||!declaredIds.has(req.id))continue;
  const key=w.world_id+"\u0000"+req.id,pair=pairId(w.world_id,req.id);
  const applies=req.tag===null||w.profile_tags.includes(req.tag),row=applicability.get(key);
  if(!row){
   if(applies)findings.push(mkFinding("MISSING_WORLD_MODEL_PAIR",req.id,pair,"required applicability row is absent from manifest"));
   else {unwitnessed++;findings.push(mkFinding("UNWITNESSED_MODEL_EXCLUSION",req.id,pair,"non-applicable model has no explicit witnessed exclusion row"));}
   continue;
  }
  if(applies&&!row.declared_applicable){unwitnessed++;findings.push(mkFinding("UNWITNESSED_MODEL_EXCLUSION",req.id,pair,"grammar requires the model in this world but manifest excludes it"));}
  else if(!applies&&row.declared_applicable)findings.push(mkFinding("UNEXPECTED_WORLD_MODEL_PAIR",req.id,pair,"manifest declares a pair excluded by profile grammar"));
  else if(!applies){
   const witnessed=row.exclusion_reason==="profile_not_applicable"&&validId(row.evidence_receipt_id);
   if(!witnessed){unwitnessed++;findings.push(mkFinding("UNWITNESSED_MODEL_EXCLUSION",req.id,pair,"profile exclusion lacks reason or evidence receipt"));}
  }
 }
 const declaredPairs=new Set(c.applicability.filter(a=>a.declared_applicable).map(a=>pairId(a.world_id,a.model_id)));
 const actualPairs=new Set();
 for(const p of c.joint_pairs){
  if(!validId(p.scenario_id)||!worlds.has(p.world_id)||!validId(p.model_id))return invalid(c,"malformed joint scenario pair");
  const key=pairId(p.world_id,p.model_id);
  if(actualPairs.has(key))return invalid(c,"duplicate joint world-model pair");
  actualPairs.add(key);
 }
 const missingPairs=new Set();
 for(const pair of expectedPairs){
  const model=pair.split("::").slice(1).join("::");
  if(!producerIds.has(model)||!declaredIds.has(model))continue;
  if(!declaredPairs.has(pair)||!actualPairs.has(pair)){missingPairs.add(pair);findings.push(mkFinding("MISSING_WORLD_MODEL_PAIR",model,pair,"independent grammar requires a world-model pair missing from manifest or joint scenarios"));}
 }
 for(const pair of actualPairs)if(!expectedPairs.has(pair)){
  const model=pair.split("::").slice(1).join("::");
  findings.push(mkFinding("UNEXPECTED_WORLD_MODEL_PAIR",model,pair,"joint scenario is outside applicability grammar"));
 }
 findings.sort((a,b)=>cmp(a.code,b.code)||cmp(a.model_id||"",b.model_id||"")||cmp(a.pair_id||"",b.pair_id||"")||cmp(a.detail,b.detail));
 const unique=[];for(const f of findings)if(!unique.length||JSON.stringify(unique.at(-1))!==JSON.stringify(f))unique.push(f);
 const codes=new Set(unique.map(f=>f.code));
 const disposition=codes.has("MODEL_STALE")?"MODEL_STALE":codes.has("MODEL_OUT_OF_PROFILE")?"MODEL_OUT_OF_PROFILE":
  codes.has("MODEL_SOURCE_UNAVAILABLE")?"MODEL_SOURCE_UNAVAILABLE":codes.has("MODEL_SCOPE_MISMATCH")?"MODEL_SCOPE_MISMATCH":
  codes.has("MODEL_RECEIPT_ALIAS")?"MODEL_RECEIPT_ALIAS":codes.has("COMMON_MODE_MODEL_OMISSION")?"COMMON_MODE_MODEL_OMISSION":
  codes.has("MISSING_LOSS_MODEL")?"MISSING_LOSS_MODEL":codes.has("UNWITNESSED_MODEL_EXCLUSION")?"UNWITNESSED_MODEL_EXCLUSION":
  codes.has("MISSING_WORLD_MODEL_PAIR")?"MISSING_WORLD_MODEL_PAIR":codes.has("UNEXPECTED_MODEL")?"UNEXPECTED_MODEL":
  codes.has("UNEXPECTED_WORLD_MODEL_PAIR")||codes.has("UNEXPECTED_APPLICABILITY")?"UNEXPECTED_WORLD_MODEL_PAIR":
  "LOSS_MODEL_UNIVERSE_CLOSED_WITHIN_DECLARED_GRAMMAR";
 const mm=[...missingModels].sort(cmp),mp=[...missingPairs].sort(cmp),um=[...unexpectedModels].sort(cmp);
 const up=[...actualPairs].filter(p=>!expectedPairs.has(p)).sort(cmp);
 return {case_id:c.case_id,disposition,expected_disposition:c.expected.disposition,missing_model_ids:mm,missing_pair_ids:mp,
  unexpected_model_ids:um,unexpected_pair_ids:up,unwitnessed_exclusion_count:unwitnessed,findings:unique,
  producer_model_count:c.producer_models.length,declared_model_count:c.declared_model_ids.length,
  expected_model_count:expectedModels.size,expected_pair_count:expectedPairs.size,actual_pair_count:actualPairs.size,
  expected_result_matched:disposition===c.expected.disposition&&JSON.stringify(mm)===JSON.stringify(c.expected.missing_model_ids)
   &&JSON.stringify(mp)===JSON.stringify(c.expected.missing_pair_ids)&&JSON.stringify(um)===JSON.stringify(c.expected.unexpected_model_ids)
   &&JSON.stringify(up)===JSON.stringify(c.expected.unexpected_pair_ids)};
}
function evaluate(corpus){
 if(corpus.schema!==schemaId||corpus.evidence_class!=="synthetic"||corpus.claim_ceiling!==ceiling||
  !Array.isArray(corpus.cases)||!corpus.cases.length||corpus.cases.length>128)throw Error("invalid loss-model adequacy corpus");
 const ids=new Set(),cases=corpus.cases.map(c=>{if(ids.has(c.case_id))throw Error("duplicate case ID");ids.add(c.case_id);return evalCase(c);}).sort((a,b)=>cmp(a.case_id,b.case_id));
 const count=s=>cases.filter(c=>c.disposition===s).length;
 const summary={case_count:cases.length,closed_cases:count("LOSS_MODEL_UNIVERSE_CLOSED_WITHIN_DECLARED_GRAMMAR"),
 common_mode_model_omission_cases:count("COMMON_MODE_MODEL_OMISSION"),missing_model_cases:count("MISSING_LOSS_MODEL"),
 missing_pair_cases:count("MISSING_WORLD_MODEL_PAIR"),unwitnessed_exclusion_cases:count("UNWITNESSED_MODEL_EXCLUSION"),
 model_family_inadequate_cases:count("MODEL_FAMILY_INADEQUATE"),stale_model_cases:count("MODEL_STALE"),
 out_of_profile_model_cases:count("MODEL_OUT_OF_PROFILE"),unexpected_model_cases:count("UNEXPECTED_MODEL"),
 unexpected_pair_cases:count("UNEXPECTED_WORLD_MODEL_PAIR"),invalid_cases:count("INVALID_CASE"),
 expected_result_mismatches:cases.filter(c=>!c.expected_result_matched).length};
 return {result:summary.expected_result_mismatches===0&&summary.closed_cases>0&&summary.common_mode_model_omission_cases>0&&summary.unwitnessed_exclusion_cases>0?"pass":"fail",
 evidence_class:"synthetic",claim_ceiling:ceiling,summary,cases};
}
try{const path=process.argv[2];if(!path)throw Error("usage: loss_model_universe_oracle.mjs <corpus.json>");
 process.stdout.write(JSON.stringify(evaluate(JSON.parse(fs.readFileSync(path,"utf8"))),null,2)+"\n");
}catch(e){process.stderr.write(String(e?.stack??e)+"\n");process.exitCode=1;}

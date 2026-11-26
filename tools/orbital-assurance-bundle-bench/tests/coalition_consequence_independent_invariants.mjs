import fs from "node:fs";
const c = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
const need = (ok, why) => { if (!ok) throw Error(why); };
const key = p => [p.scenario_id,p.world_id,p.model_id ?? p.loss_model_id].join("|");
const sorted = xs => [...xs].sort();
for (const x of c.cases) {
  const b=x.incumbent_bridge_case, m=x.loss_model_universe_case, lm=b.loss_model_set;
  const req=m.mission_class==="orbital_vehicle"
    ? [["propellant-v1","propellant_consumption",null],["thermal-v1","thermal_stress","thermal"],["propulsion-v1","propulsion_degradation","propulsion"],["avionics-v1","avionics_reliability",null]]
    : m.mission_class==="powered_flight"
      ? [["propellant-v1","propellant_consumption",null],["thermal-v1","thermal_stress","thermal"],["avionics-v1","avionics_reliability",null]] : null;
  need(req,"mission class absent from independent family map");
  const ids=req.map(r=>r[0]);
  need(sorted(m.declared_model_ids).join("|")===sorted(ids).join("|"),"independent declared-model set mismatch");
  need(sorted(m.producer_models.map(r=>r.model_id)).join("|")===sorted(ids).join("|"),"independent producer-model set mismatch");
  for (const [id,family] of req) {
    const p=m.producer_models.find(v=>v.model_id===id);
    need(p && p.family===family && p.state==="CURRENT" && p.scope_sha256===x.scope_sha256 && p.source_receipt_id,"model family/source pin mismatch");
  }
  const applicable=[];
  for (const w of m.worlds) {
    need(w.scope_sha256===x.scope_sha256,"world scope mismatch");
    for (const [id,,tag] of req) {
      const row=m.applicability.find(a=>a.world_id===w.world_id && a.model_id===id);
      need(row,"missing world/model applicability row");
      const yes=tag===null || w.profile_tags.includes(tag);
      need(yes ? row.declared_applicable===true : row.declared_applicable===false && row.exclusion_reason==="profile_not_applicable" && row.evidence_receipt_id,"invalid applicability/exclusion evidence");
      if(yes) applicable.push(w.world_id+"|"+id);
    }
  }
  need(m.applicability.length===m.worlds.length*req.length,"extra/duplicate applicability rows");
  const expected=sorted(m.joint_pairs.map(p=>p.world_id+"|"+p.model_id));
  need(expected.join("|")===sorted(applicable).join("|"),"independent world/model pair closure mismatch");
  const modelTuples=m.joint_pairs.map(key), bridgeTuples=b.worlds.map(key);
  need(modelTuples.length===new Set(modelTuples).size && bridgeTuples.length===new Set(bridgeTuples).size,"duplicate joint tuple");
  need(sorted(modelTuples).join("\n")===sorted(bridgeTuples).join("\n"),"scenario/world/model tuple sets differ");
  need(lm.projection_id==="exact-rational-to-integer-identity-v1" && lm.projection_scale_numerator===1 && lm.projection_scale_denominator===1,"unsupported integer projection in independent check");
  for(const w of b.worlds)for(const loss of w.losses) {
    need(Number.isSafeInteger(loss.source_numerator) && Number.isSafeInteger(loss.source_denominator) && loss.source_denominator>0,"source rational outside exact integer bounds");
    need(loss.source_numerator%loss.source_denominator===0 && loss.source_numerator/loss.source_denominator===loss.loss,"source rational projection mismatch");
  }
}
process.stdout.write("coalition-consequence-independent-source-invariants-valid\n");

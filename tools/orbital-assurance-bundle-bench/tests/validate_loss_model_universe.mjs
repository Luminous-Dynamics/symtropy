import fs from "node:fs";
const [schemaPath,corpusPath]=process.argv.slice(2);
if(!schemaPath||!corpusPath)throw Error("usage: validate_loss_model_universe.mjs <schema> <corpus>");
const schema=JSON.parse(fs.readFileSync(schemaPath,"utf8")),corpus=JSON.parse(fs.readFileSync(corpusPath,"utf8"));
if(schema.properties?.schema?.const!=="luminous.orbital-loss-model-universe-adequacy.corpus.v0.1"
 ||schema.properties?.claim_ceiling?.const!=="synthetic-loss-model-universe-adequacy-only")throw Error("loss-model universe schema contract malformed");
if(corpus.schema!==schema.properties.schema.const||corpus.claim_ceiling!==schema.properties.claim_ceiling.const
 ||corpus.evidence_class!=="synthetic"||!Array.isArray(corpus.cases)||corpus.cases.length!==17)throw Error("loss-model universe corpus contract mismatch");
const base=corpus.cases.find(c=>c.case_id==="baseline_closed");
const common=corpus.cases.find(c=>c.case_id==="common_mode_omission");
const valid=corpus.cases.find(c=>c.case_id==="valid_exclusion");
const unwitnessed=corpus.cases.find(c=>c.case_id==="unwitnessed_exclusion");
if(!base||!common||!valid||!unwitnessed)throw Error("required model adequacy controls missing");
if(common.expected.disposition!=="COMMON_MODE_MODEL_OMISSION"||common.producer_models.some(m=>m.model_id==="thermal-v1")
 ||common.declared_model_ids.includes("thermal-v1"))throw Error("common-mode omission control malformed");
const exclusions=valid.applicability.filter(a=>!a.declared_applicable);
if(!exclusions.length||exclusions.some(a=>a.exclusion_reason!=="profile_not_applicable"||!a.evidence_receipt_id))throw Error("valid profile exclusion is not witnessed");
if(!unwitnessed.applicability.some(a=>!a.declared_applicable&&!a.evidence_receipt_id))throw Error("unwitnessed exclusion control malformed");
process.stdout.write("loss-model-universe-corpus-valid: "+corpus.cases.length+" cases\n");

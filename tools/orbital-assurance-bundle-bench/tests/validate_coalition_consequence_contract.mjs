import fs from "node:fs";
const [schemaPath, corpusPath] = process.argv.slice(2);
if (!schemaPath || !corpusPath) throw Error("usage: validate_coalition_consequence_contract.mjs <schema> <corpus>");
const schema = JSON.parse(fs.readFileSync(schemaPath, "utf8"));
const corpus = JSON.parse(fs.readFileSync(corpusPath, "utf8"));
const need = (ok, why) => { if (!ok) throw Error(why); };
need(schema.$schema === "https://json-schema.org/draft/2020-12/schema", "wrong schema dialect");
need(corpus.schema === "luminous.orbital-coalition-consequence-binding.corpus.v0.1", "wrong corpus schema");
need(corpus.evidence_class === "synthetic" && corpus.claim_ceiling === "synthetic-finite-joint-scenario-coalition-binding-only", "claim ceiling mismatch");
need(Array.isArray(corpus.cases) && corpus.cases.length > 0 && corpus.cases.length <= 128, "case count out of bounds");
const ids = new Set();
for (const c of corpus.cases) {
  need(typeof c.case_id === "string" && !ids.has(c.case_id), "missing or duplicate case ID"); ids.add(c.case_id);
  need(/^[0-9a-f]{64}$/.test(c.scope_sha256) && typeof c.loss_unit === "string", "scope or unit invalid");
  need(Array.isArray(c.candidate_actions) && c.candidate_actions.length > 0 && c.candidate_actions.length <= 16, "candidate action bound invalid");
  need(Array.isArray(c.coalitions) && c.coalitions.length > 0, "coalition outcomes missing");
  need(c.incumbent_bridge_case && c.loss_model_universe_case && /^[0-9a-f]{40}$/.test(c.loss_model_universe_checker_commit), "upstream evidence pins missing");
}
process.stdout.write("coalition-consequence-corpus-contract-valid\n");

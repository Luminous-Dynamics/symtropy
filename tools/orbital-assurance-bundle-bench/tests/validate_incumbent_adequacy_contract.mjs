import fs from "node:fs";

const [schemaPath, corpusPath] = process.argv.slice(2);
if (!schemaPath || !corpusPath) throw new Error("usage: validate_incumbent_adequacy_contract.mjs <schema> <corpus>");
const schema = JSON.parse(fs.readFileSync(schemaPath, "utf8"));
const corpus = JSON.parse(fs.readFileSync(corpusPath, "utf8"));
if (schema.$schema !== "https://json-schema.org/draft/2020-12/schema"
  || schema.properties?.schema?.const !== "luminous.orbital-incumbent-adequacy.corpus.v0.1"
  || schema.properties?.evidence_class?.const !== "synthetic"
  || schema.properties?.claim_ceiling?.const !== "synthetic-finite-world-incumbent-adequacy-only") {
  throw new Error("incumbent adequacy corpus schema contract is malformed");
}
if (corpus.schema !== schema.properties.schema.const
  || corpus.evidence_class !== schema.properties.evidence_class.const
  || corpus.claim_ceiling !== schema.properties.claim_ceiling.const
  || !Array.isArray(corpus.cases) || corpus.cases.length !== 13) {
  throw new Error("incumbent adequacy corpus does not match its v0.1 contract");
}
const ids = new Set();
for (const item of corpus.cases) {
  if (!item.case_id || ids.has(item.case_id) || !Array.isArray(item.action_ids)
    || !Array.isArray(item.worlds) || !item.expected) {
    throw new Error("invalid incumbent adequacy case structure");
  }
  ids.add(item.case_id);
  for (const world of item.worlds) {
    if (!/^[0-9a-f]{64}$/.test(world.scope_sha256)
      || !Array.isArray(world.feasible_action_ids) || !Array.isArray(world.losses)) {
      throw new Error("invalid world scope or loss matrix in " + item.case_id);
    }
    for (const row of world.losses) {
      if (!Number.isSafeInteger(row.loss) || row.loss < 0 || !row.action_id) {
        throw new Error("invalid loss row in " + item.case_id + "/" + world.world_id);
      }
    }
  }
}
process.stdout.write("incumbent-adequacy-corpus-valid: " + corpus.cases.length + " cases\n");

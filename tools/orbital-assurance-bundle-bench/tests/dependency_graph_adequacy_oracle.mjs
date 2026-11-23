import fs from "node:fs";

const SCHEMA = "luminous.orbital-dependency-graph-adequacy.corpus.v0.1";
const CLAIM_CEILING = "synthetic-dependency-graph-adequacy-only";
const MAX_CASES = 128;

const grammar = {
  build_closure: ["domain_manifest"],
  build_domain_oracle: ["domain_manifest"],
  evaluate_feasibility: ["closure_report", "domain_oracle_report"],
  select_query_policy: ["feasibility_report", "domain_oracle_report"],
  build_loss_model: ["closure_report"],
  compute_regret: ["feasibility_report", "loss_model"],
  calibrate_regret: ["regret_report", "loss_model"],
  compose_decision_frontier: ["query_policy", "regret_report", "calibration"],
  ingest_campaign: []
};

function validId(value) {
  return typeof value === "string" && value.trim().length > 0
    && [...value].every(char => {
      const code = char.codePointAt(0);
      return (code >= 33 && code <= 126) || code === 32;
    });
}
function validSha(value) { return typeof value === "string" && /^[0-9a-f]{64}$/.test(value); }
function compare(left, right) { return left < right ? -1 : left > right ? 1 : 0; }
function edgeLabel(edge) { return edge[0] + "->" + edge[1] + "#" + edge[2]; }
function edgeKey(edge) { return JSON.stringify(edge); }
function addFinding(findings, code, edge, detail) {
  findings.push({ code, dependency_edge: edge ?? null, detail });
}
function invalidGraph(item, detail) {
  const findings = [{ code: "INVALID_GRAPH", dependency_edge: null, detail }];
  return {
    case_id: item.case_id,
    disposition: "INVALID_GRAPH",
    expected_disposition: item.expected.disposition,
    missing_dependency_edges: [],
    extra_dependency_edges: [],
    findings,
    producer_count: item.producers.length,
    consumer_count: item.consumers.length,
    declared_edge_count: item.declared_edges.length,
    expected_edge_count: 0,
    expected_result_matched: item.expected.disposition === "INVALID_GRAPH"
      && item.expected.missing_dependency_edges.length === 0
      && item.expected.extra_dependency_edges.length === 0
  };
}
function graphHasCycle(nodeIds, edges) {
  const indegree = new Map([...nodeIds].map(id => [id, 0]));
  const children = new Map();
  for (const edge of edges) {
    const [parent, child] = edge;
    if (!indegree.has(parent) || !indegree.has(child)) return true;
    indegree.set(child, indegree.get(child) + 1);
    if (!children.has(parent)) children.set(parent, []);
    children.get(parent).push(child);
  }
  const ready = [...indegree].filter(([, count]) => count === 0).map(([id]) => id).sort(compare);
  let visited = 0;
  while (ready.length) {
    const id = ready.shift();
    visited += 1;
    for (const child of children.get(id) || []) {
      const count = indegree.get(child) - 1;
      indegree.set(child, count);
      if (count === 0) {
        ready.push(child);
        ready.sort(compare);
      }
    }
  }
  return visited !== nodeIds.size;
}
function evaluateCase(item) {
  if (!validId(item.case_id) || !item.producers.length || item.producers.length > 4096
    || !item.consumers.length || item.consumers.length > 4096 || item.declared_edges.length > 16384
    || !["GRAPH_COMPLETE_WITHIN_DECLARED_GRAMMAR", "MISSING_DEPENDENCY_EDGE",
      "COMMON_MODE_DEPENDENCY_OMISSION", "CONSUMER_MANIFEST_OMISSION",
      "UNDECLARED_DEPENDENCY_EDGE", "DEPENDENCY_MODEL_INADEQUATE",
      "DEPENDENCY_SOURCE_UNAVAILABLE", "DEPENDENCY_SCOPE_MISMATCH", "INVALID_GRAPH"]
      .includes(item.expected.disposition)) {
    return invalidGraph(item, "invalid case identity or exceeded bounds");
  }

  const producersByNode = new Map();
  const producerByArtifact = new Map();
  const producersByType = new Map();
  const nodeIds = new Set();
  for (const producer of item.producers) {
    if (!validId(producer.node_id) || !validId(producer.artifact_id)
      || !validId(producer.artifact_type) || !validSha(producer.content_sha256)
      || !validSha(producer.scope_sha256) || producersByNode.has(producer.node_id)
      || producerByArtifact.has(producer.artifact_id)) {
      return invalidGraph(item, "duplicate producer identity or malformed artifact binding");
    }
    producersByNode.set(producer.node_id, producer);
    producerByArtifact.set(producer.artifact_id, producer);
    nodeIds.add(producer.node_id);
    if (!producersByType.has(producer.artifact_type)) producersByType.set(producer.artifact_type, []);
    producersByType.get(producer.artifact_type).push(producer);
  }

  const consumersByNode = new Map();
  const findings = [];
  for (const consumer of item.consumers) {
    if (!validId(consumer.node_id) || !validId(consumer.output_artifact_id)
      || !validId(consumer.operation) || !validSha(consumer.scope_sha256)
      || consumer.input_artifact_ids.some(id => !validId(id))
      || consumersByNode.has(consumer.node_id)) {
      return invalidGraph(item, "duplicate consumer identity or malformed consumer manifest");
    }
    const output = producersByNode.get(consumer.node_id);
    if (!output) return invalidGraph(item, "consumer output has no producer record");
    if (output.artifact_id !== consumer.output_artifact_id) {
      return invalidGraph(item, "consumer output artifact differs from producer manifest");
    }
    if (output.scope_sha256 !== consumer.scope_sha256) {
      addFinding(findings, "DEPENDENCY_SCOPE_MISMATCH", "output:" + consumer.node_id,
        "consumer-produced artifact scope differs from its transformation scope");
    }
    const inputs = new Set();
    for (const id of consumer.input_artifact_ids) {
      if (inputs.has(id)) return invalidGraph(item, "consumer input manifest contains duplicate artifact IDs");
      inputs.add(id);
    }
    consumersByNode.set(consumer.node_id, consumer);
  }

  const declared = new Map();
  for (const edge of item.declared_edges) {
    if (!validId(edge.parent_node_id) || !validId(edge.consumer_node_id) || !validId(edge.artifact_id)
      || !producersByNode.has(edge.parent_node_id) || !consumersByNode.has(edge.consumer_node_id)
      || producersByNode.get(edge.parent_node_id).artifact_id !== edge.artifact_id
      || edge.parent_node_id === edge.consumer_node_id) {
      return invalidGraph(item, "declared edge has missing endpoint or mismatched artifact");
    }
    const key = [edge.parent_node_id, edge.consumer_node_id, edge.artifact_id];
    const serialized = edgeKey(key);
    if (declared.has(serialized)) return invalidGraph(item, "declared dependency graph contains duplicate edges");
    declared.set(serialized, key);
  }

  if (graphHasCycle(nodeIds, [...declared.values()])) {
    return invalidGraph(item, "declared dependency graph contains a cycle");
  }

  const expected = new Map();
  const missing = new Set();
  const extra = new Set();
  for (const consumer of item.consumers) {
    const requiredTypes = grammar[consumer.operation];
    if (requiredTypes === undefined) {
      addFinding(findings, "DEPENDENCY_MODEL_INADEQUATE", null,
        "operation " + consumer.operation + " has no registered dependency grammar");
      continue;
    }
    const inputIds = new Set(consumer.input_artifact_ids);
    for (const parentType of requiredTypes) {
      const parents = producersByType.get(parentType) || [];
      if (parents.length !== 1) {
        addFinding(findings, "DEPENDENCY_SOURCE_UNAVAILABLE", null,
          "operation " + consumer.operation + " requires exactly one producer of type "
            + parentType + ", found " + parents.length);
        continue;
      }
      const parent = parents[0];
      if (parent.scope_sha256 !== consumer.scope_sha256) {
        addFinding(findings, "DEPENDENCY_SCOPE_MISMATCH", parent.node_id + "->" + consumer.node_id,
          "producer and consumer scope digests differ");
      }
      const key = [parent.node_id, consumer.node_id, parent.artifact_id];
      const serialized = edgeKey(key);
      expected.set(serialized, key);
      const label = edgeLabel(key);
      const manifestHas = inputIds.has(parent.artifact_id);
      const graphHas = declared.has(serialized);
      if (!manifestHas && !graphHas) {
        addFinding(findings, "COMMON_MODE_DEPENDENCY_OMISSION", label,
          "both consumer input manifest and declared graph omit a grammar-required parent");
        missing.add(label);
      } else if (!manifestHas) {
        addFinding(findings, "CONSUMER_MANIFEST_OMISSION", label,
          "consumer input manifest omits a grammar-required artifact while graph edge exists");
      } else if (!graphHas) {
        addFinding(findings, "MISSING_DEPENDENCY_EDGE", label,
          "consumer input manifest declares the artifact but assurance graph omits its edge");
        missing.add(label);
      }
    }
    for (const artifactId of consumer.input_artifact_ids) {
      const required = [...expected.values()].some(edge =>
        edge[1] === consumer.node_id && edge[2] === artifactId);
      if (!required) {
        addFinding(findings, "UNDECLARED_DEPENDENCY_EDGE", "artifact:" + artifactId + "->" + consumer.node_id,
          "consumer declares an input not required by the registered transformation grammar");
      }
    }
  }

  for (const [serialized, edge] of declared) {
    if (!expected.has(serialized)) {
      const label = edgeLabel(edge);
      extra.add(label);
      addFinding(findings, "UNDECLARED_DEPENDENCY_EDGE", label,
        "assurance graph contains an edge not required by the registered transformation grammar");
    }
  }

  findings.sort((a, b) => compare(a.code, b.code)
    || compare(a.dependency_edge || "", b.dependency_edge || "")
    || compare(a.detail, b.detail));
  const unique = [];
  for (const finding of findings) {
    if (!unique.length || JSON.stringify(unique[unique.length - 1]) !== JSON.stringify(finding)) unique.push(finding);
  }
  const codes = new Set(unique.map(finding => finding.code));
  let disposition = "GRAPH_COMPLETE_WITHIN_DECLARED_GRAMMAR";
  if (codes.has("INVALID_GRAPH")) disposition = "INVALID_GRAPH";
  else if (codes.has("DEPENDENCY_MODEL_INADEQUATE")) disposition = "DEPENDENCY_MODEL_INADEQUATE";
  else if (codes.has("DEPENDENCY_SOURCE_UNAVAILABLE")) disposition = "DEPENDENCY_SOURCE_UNAVAILABLE";
  else if (codes.has("DEPENDENCY_SCOPE_MISMATCH")) disposition = "DEPENDENCY_SCOPE_MISMATCH";
  else if (codes.has("COMMON_MODE_DEPENDENCY_OMISSION")) disposition = "COMMON_MODE_DEPENDENCY_OMISSION";
  else if (codes.has("CONSUMER_MANIFEST_OMISSION")) disposition = "CONSUMER_MANIFEST_OMISSION";
  else if (codes.has("MISSING_DEPENDENCY_EDGE")) disposition = "MISSING_DEPENDENCY_EDGE";
  else if (codes.has("UNDECLARED_DEPENDENCY_EDGE")) disposition = "UNDECLARED_DEPENDENCY_EDGE";

  const missingEdges = [...missing].sort(compare);
  const extraEdges = [...extra].sort(compare);
  const expectedMissing = new Set(item.expected.missing_dependency_edges);
  const expectedExtra = new Set(item.expected.extra_dependency_edges);
  const expectedResultMatched = disposition === item.expected.disposition
    && expectedMissing.size === missingEdges.length
    && missingEdges.every(edge => expectedMissing.has(edge))
    && expectedExtra.size === extraEdges.length
    && extraEdges.every(edge => expectedExtra.has(edge))
    && item.expected.missing_dependency_edges.length === expectedMissing.size
    && item.expected.extra_dependency_edges.length === expectedExtra.size;

  return {
    case_id: item.case_id, disposition, expected_disposition: item.expected.disposition,
    missing_dependency_edges: missingEdges, extra_dependency_edges: extraEdges, findings: unique,
    producer_count: item.producers.length, consumer_count: item.consumers.length,
    declared_edge_count: declared.size, expected_edge_count: expected.size, expected_result_matched: expectedResultMatched
  };
}
function evaluate(corpus) {
  if (corpus.schema !== SCHEMA || corpus.evidence_class !== "synthetic"
    || corpus.claim_ceiling !== CLAIM_CEILING || !Array.isArray(corpus.cases)
    || !corpus.cases.length || corpus.cases.length > MAX_CASES) {
    throw new Error("unsupported dependency graph adequacy corpus or claim ceiling");
  }
  const ids = new Set();
  const cases = corpus.cases.map(item => {
    if (ids.has(item.case_id)) throw new Error("duplicate dependency graph case ID");
    ids.add(item.case_id);
    return evaluateCase(item);
  }).sort((a, b) => compare(a.case_id, b.case_id));
  const count = disposition => cases.filter(item => item.disposition === disposition).length;
  const summary = {
    case_count: cases.length,
    graph_complete_cases: count("GRAPH_COMPLETE_WITHIN_DECLARED_GRAMMAR"),
    missing_edge_cases: count("MISSING_DEPENDENCY_EDGE"),
    common_mode_omission_cases: count("COMMON_MODE_DEPENDENCY_OMISSION"),
    consumer_manifest_omission_cases: count("CONSUMER_MANIFEST_OMISSION"),
    extra_edge_cases: count("UNDECLARED_DEPENDENCY_EDGE"),
    model_inadequate_cases: count("DEPENDENCY_MODEL_INADEQUATE"),
    source_unavailable_cases: count("DEPENDENCY_SOURCE_UNAVAILABLE"),
    scope_mismatch_cases: count("DEPENDENCY_SCOPE_MISMATCH"),
    invalid_graph_cases: count("INVALID_GRAPH"),
    missing_dependency_edge_count: cases.reduce((sum, item) => sum + item.missing_dependency_edges.length, 0),
    extra_dependency_edge_count: cases.reduce((sum, item) => sum + item.extra_dependency_edges.length, 0),
    expected_result_mismatches: cases.filter(item => !item.expected_result_matched).length
  };
  const pass = summary.expected_result_mismatches === 0
    && summary.graph_complete_cases >= 1 && summary.common_mode_omission_cases >= 1;
  return {
    result: pass ? "pass" : "fail",
    evidence_class: "synthetic",
    claim_ceiling: CLAIM_CEILING,
    summary,
    cases
  };
}

try {
  const path = process.argv[2];
  if (!path) throw new Error("usage: dependency_graph_adequacy_oracle.mjs <cases.json>");
  process.stdout.write(JSON.stringify(evaluate(JSON.parse(fs.readFileSync(path, "utf8"))), null, 2) + "\n");
} catch (error) {
  process.stderr.write(String(error?.stack ?? error) + "\n");
  process.exitCode = 1;
}

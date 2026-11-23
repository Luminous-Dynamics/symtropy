import fs from "node:fs";

const SCHEMA = "luminous.orbital-assurance-invalidation.corpus.v0.1";
const CLAIM_CEILING = "synthetic-dependency-invalidation-only";

function validId(value) {
  return typeof value === "string" && value.trim().length > 0 && /^[\x00-\x7F]+$/.test(value);
}
function validSha(value) { return typeof value === "string" && /^[0-9a-f]{64}$/.test(value); }
function invalidGraph(item, detail) {
  const nodeIds = [...new Set(item.nodes.map(node => node.node_id))].sort();
  const roots = item.required_root_node_ids.filter(id => nodeIds.includes(id));
  return {
    case_id: item.case_id,
    disposition: "INVALID_GRAPH",
    required_root_node_ids: item.required_root_node_ids,
    invalidated_required_root_node_ids: roots,
    invalidated_node_ids: nodeIds,
    current_node_count: 0,
    invalidated_node_count: nodeIds.length,
    nodes: nodeIds.map(node_id => ({
      node_id, status: "INVALID_GRAPH", invalidated_by: [detail]
    })),
    authority_granted: false,
    expected_result_matched: item.expected_disposition === "INVALID_GRAPH"
      && new Set(item.expected_invalidated_node_ids).size === new Set(nodeIds).size
      && nodeIds.every(id => item.expected_invalidated_node_ids.includes(id))
  };
}

function evaluateCase(item) {
  if (!validId(item.case_id) || !item.nodes.length || item.nodes.length > 4096
    || !item.required_root_node_ids.length || item.required_root_node_ids.length > 4096
    || item.expected_invalidated_node_ids.length > 4096
    || !["CURRENT", "REQUALIFICATION_REQUIRED", "INVALID_GRAPH"].includes(item.expected_disposition)) {
    return invalidGraph(item, "invalid case identity or bounds");
  }

  const nodeIndex = new Map();
  let dependencyCount = 0;
  for (const [index, node] of item.nodes.entries()) {
    if (!validId(node.node_id) || !validId(node.node_type)
      || !validSha(node.content_sha256) || !validSha(node.current_content_sha256)
      || !validSha(node.scope_sha256) || !validSha(node.current_scope_sha256)
      || node.dependencies.length > 4096 || nodeIndex.has(node.node_id)) {
      return invalidGraph(item, "duplicate node or malformed node identity");
    }
    nodeIndex.set(node.node_id, index);
    dependencyCount += node.dependencies.length;
    if (dependencyCount > 16384) return invalidGraph(item, "dependency inventory exceeds bound");
  }

  const roots = new Set();
  for (const rootId of item.required_root_node_ids) {
    if (!validId(rootId) || roots.has(rootId) || !nodeIndex.has(rootId)) {
      return invalidGraph(item, "required root is missing, empty or duplicated");
    }
    roots.add(rootId);
  }

  const children = new Map();
  const indegree = new Map(item.nodes.map(node => [node.node_id, node.dependencies.length]));
  for (const node of item.nodes) {
    const dependencyIds = new Set();
    for (const dependency of node.dependencies) {
      if (!validId(dependency.node_id) || !validSha(dependency.expected_content_sha256)
        || !validSha(dependency.expected_scope_sha256) || dependencyIds.has(dependency.node_id)
        || !nodeIndex.has(dependency.node_id) || dependency.node_id === node.node_id) {
        return invalidGraph(item, "missing, duplicated or malformed dependency pin");
      }
      dependencyIds.add(dependency.node_id);
      if (!children.has(dependency.node_id)) children.set(dependency.node_id, []);
      children.get(dependency.node_id).push(node.node_id);
    }
  }

  // Independent Kahn topological validation. A cycle invalidates the entire graph.
  const ready = [...indegree].filter(([, count]) => count === 0).map(([id]) => id).sort();
  let visited = 0;
  while (ready.length) {
    const id = ready.shift();
    visited += 1;
    for (const child of children.get(id) || []) {
      const count = indegree.get(child) - 1;
      indegree.set(child, count);
      if (count === 0) {
        ready.push(child);
        ready.sort();
      }
    }
  }
  if (visited !== item.nodes.length) return invalidGraph(item, "dependency graph contains a cycle");

  const statuses = new Map();
  const invalidatedBy = new Map();
  const queue = [];
  const addCause = (id, cause) => {
    if (!invalidatedBy.has(id)) invalidatedBy.set(id, new Set());
    invalidatedBy.get(id).add(cause);
  };

  for (const node of item.nodes) {
    let status = null;
    if (node.content_sha256 !== node.current_content_sha256) status = "SOURCE_CHANGED";
    else if (node.scope_sha256 !== node.current_scope_sha256) status = "SCOPE_CHANGED";
    if (status) {
      statuses.set(node.node_id, status);
      addCause(node.node_id, node.node_id);
      queue.push(node.node_id);
    }
  }

  for (const node of item.nodes) {
    for (const dependency of node.dependencies) {
      const parent = item.nodes[nodeIndex.get(dependency.node_id)];
      if (dependency.expected_content_sha256 !== parent.current_content_sha256
        || dependency.expected_scope_sha256 !== parent.current_scope_sha256) {
        if (!statuses.has(node.node_id)) statuses.set(node.node_id, "DEPENDENCY_VERSION_MISMATCH");
        addCause(node.node_id, parent.node_id);
        queue.push(node.node_id);
      }
    }
  }

  const propagated = new Set();
  while (queue.length) {
    const parentId = queue.shift();
    if (propagated.has(parentId)) continue;
    propagated.add(parentId);
    for (const childId of children.get(parentId) || []) {
      addCause(childId, parentId);
      if (!statuses.has(childId)) {
        statuses.set(childId, "UPSTREAM_INVALIDATED");
        queue.push(childId);
      } else if (!propagated.has(childId)) {
        queue.push(childId);
      }
    }
  }

  const nodes = item.nodes.map(node => ({
    node_id: node.node_id,
    status: statuses.get(node.node_id) || "CURRENT",
    invalidated_by: [...(invalidatedBy.get(node.node_id) || [])].sort()
  })).sort((left, right) => left.node_id.localeCompare(right.node_id));
  const invalidatedNodeIds = nodes.filter(node => node.status !== "CURRENT").map(node => node.node_id);
  const invalidatedRoots = item.required_root_node_ids.filter(id => statuses.has(id));
  const disposition = invalidatedRoots.length ? "REQUALIFICATION_REQUIRED" : "CURRENT";
  const expectedInvalidated = new Set(item.expected_invalidated_node_ids);
  const actualInvalidated = new Set(invalidatedNodeIds);
  const expectedRoots = [...item.required_root_node_ids].sort();
  const actualRoots = [...new Set(item.required_root_node_ids)].sort();
  const expectedResultMatched = expectedRoots.length === actualRoots.length
    && expectedInvalidated.size === actualInvalidated.size
    && [...expectedInvalidated].every(id => actualInvalidated.has(id))
    && disposition === item.expected_disposition;

  return {
    case_id: item.case_id,
    disposition,
    required_root_node_ids: item.required_root_node_ids,
    invalidated_required_root_node_ids: invalidatedRoots,
    invalidated_node_ids: invalidatedNodeIds,
    current_node_count: nodes.length - invalidatedNodeIds.length,
    invalidated_node_count: invalidatedNodeIds.length,
    nodes,
    authority_granted: false,
    expected_result_matched: expectedResultMatched
  };
}

function evaluate(corpus) {
  if (corpus.schema !== SCHEMA || corpus.evidence_class !== "synthetic"
    || corpus.claim_ceiling !== CLAIM_CEILING || !Array.isArray(corpus.cases)
    || !corpus.cases.length || corpus.cases.length > 128) {
    throw new Error("unsupported assurance-invalidation corpus or claim ceiling");
  }
  const ids = new Set();
  const cases = corpus.cases.map(item => {
    if (ids.has(item.case_id)) throw new Error("duplicate case ID");
    ids.add(item.case_id);
    return evaluateCase(item);
  }).sort((a, b) => a.case_id.localeCompare(b.case_id));
  const count = disposition => cases.filter(item => item.disposition === disposition).length;
  const nodeCount = status => cases.flatMap(item => item.nodes)
    .filter(node => node.status === status).length;
  const summary = {
    case_count: cases.length,
    current_cases: count("CURRENT"),
    requalification_required_cases: count("REQUALIFICATION_REQUIRED"),
    invalid_graph_cases: count("INVALID_GRAPH"),
    source_changed_nodes: nodeCount("SOURCE_CHANGED"),
    scope_changed_nodes: nodeCount("SCOPE_CHANGED"),
    dependency_version_mismatch_nodes: nodeCount("DEPENDENCY_VERSION_MISMATCH"),
    upstream_invalidated_nodes: nodeCount("UPSTREAM_INVALIDATED"),
    invalidated_required_roots: cases.reduce((sum, item) => sum + item.invalidated_required_root_node_ids.length, 0),
    expected_result_mismatches: cases.filter(item => !item.expected_result_matched).length,
    authority_grants: cases.filter(item => item.authority_granted).length
  };
  return {
    result: summary.expected_result_mismatches === 0 && summary.authority_grants === 0 ? "pass" : "fail",
    evidence_class: "synthetic",
    claim_ceiling: CLAIM_CEILING,
    summary,
    cases,
    authority_granted: false
  };
}

try {
  const path = process.argv[2];
  if (!path) throw new Error("usage: dependency_invalidation_oracle.mjs <cases.json>");
  process.stdout.write(JSON.stringify(evaluate(JSON.parse(fs.readFileSync(path, "utf8"))), null, 2) + "\n");
} catch (error) {
  process.stderr.write(String(error?.stack ?? error) + "\n");
  process.exitCode = 1;
}

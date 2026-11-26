use single_commodity_flow_kernel::{
    FlowCertificateBundle, FlowEdge, FlowGraph, FlowNode, MAX_EDGES, MAX_NODES,
};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::process;

const DIMACS_MAX_CAPACITY: u64 = i32::MAX as u64;

fn parse_dimacs_max(input: &str) -> Result<FlowGraph, String> {
    let mut problem: Option<(usize, usize)> = None;
    let mut source: Option<usize> = None;
    let mut sink: Option<usize> = None;
    let mut arcs: Vec<(usize, usize, u64, usize)> = Vec::new();
    let mut saw_arc = false;

    for (line_zero, raw_line) in input.lines().enumerate() {
        let line_number = line_zero + 1;
        let tokens = raw_line.split_whitespace().collect::<Vec<_>>();
        if tokens.is_empty() {
            continue;
        }

        match tokens[0] {
            "c" => continue,
            "p" => {
                if problem.is_some() {
                    return Err(format!("line {line_number}: duplicate problem line"));
                }
                if source.is_some() || sink.is_some() || saw_arc {
                    return Err(format!("line {line_number}: problem line must precede node/arc lines"));
                }
                if tokens.len() != 4 || tokens[1] != "max" {
                    return Err(format!("line {line_number}: expected p max <n> <m>"));
                }
                let node_count = tokens[2]
                    .parse::<usize>()
                    .map_err(|_| format!("line {line_number}: invalid node count"))?;
                let arc_count = tokens[3]
                    .parse::<usize>()
                    .map_err(|_| format!("line {line_number}: invalid arc count"))?;
                if node_count < 2 || node_count > MAX_NODES {
                    return Err(format!(
                        "line {line_number}: node count outside supported kernel range 2..={MAX_NODES}"
                    ));
                }
                if arc_count > MAX_EDGES {
                    return Err(format!(
                        "line {line_number}: arc count exceeds supported kernel limit {MAX_EDGES}"
                    ));
                }
                problem = Some((node_count, arc_count));
            }
            "n" => {
                let (node_count, _) = problem
                    .ok_or_else(|| format!("line {line_number}: node line before problem line"))?;
                if saw_arc {
                    return Err(format!("line {line_number}: node lines must precede arc lines"));
                }
                if tokens.len() != 3 {
                    return Err(format!("line {line_number}: expected n <v> <s|t>"));
                }
                let node_id = tokens[1]
                    .parse::<usize>()
                    .map_err(|_| format!("line {line_number}: invalid node ID"))?;
                if !(1..=node_count).contains(&node_id) {
                    return Err(format!("line {line_number}: node ID outside 1..={node_count}"));
                }
                match tokens[2] {
                    "s" => {
                        if source.replace(node_id).is_some() {
                            return Err(format!("line {line_number}: duplicate source declaration"));
                        }
                    }
                    "t" => {
                        if sink.replace(node_id).is_some() {
                            return Err(format!("line {line_number}: duplicate sink declaration"));
                        }
                    }
                    _ => return Err(format!("line {line_number}: node marker must be s or t")),
                }
            }
            "a" => {
                let (node_count, arc_count) = problem
                    .ok_or_else(|| format!("line {line_number}: arc line before problem line"))?;
                saw_arc = true;
                if tokens.len() != 4 {
                    return Err(format!("line {line_number}: expected a <v> <w> <capacity>"));
                }
                let from = tokens[1]
                    .parse::<usize>()
                    .map_err(|_| format!("line {line_number}: invalid arc tail"))?;
                let to = tokens[2]
                    .parse::<usize>()
                    .map_err(|_| format!("line {line_number}: invalid arc head"))?;
                if !(1..=node_count).contains(&from) || !(1..=node_count).contains(&to) {
                    return Err(format!("line {line_number}: arc endpoint outside 1..={node_count}"));
                }
                if from == to {
                    return Err(format!("line {line_number}: self-loop arcs are unsupported"));
                }
                let capacity = tokens[3]
                    .parse::<u64>()
                    .map_err(|_| format!("line {line_number}: invalid non-negative capacity"))?;
                if !(1..=DIMACS_MAX_CAPACITY).contains(&capacity) {
                    return Err(format!(
                        "line {line_number}: DIMACS .max capacity outside 1..={DIMACS_MAX_CAPACITY}"
                    ));
                }
                if arcs.len() >= arc_count {
                    return Err(format!("line {line_number}: more arcs than declared"));
                }
                arcs.push((from, to, capacity, line_number));
            }
            other => {
                return Err(format!("line {line_number}: unsupported record type {other}"));
            }
        }
    }

    let (node_count, declared_arcs) =
        problem.ok_or_else(|| "missing DIMACS problem line".to_string())?;
    if arcs.len() != declared_arcs {
        return Err(format!(
            "declared {declared_arcs} arcs but parsed {}",
            arcs.len()
        ));
    }
    let source = source.ok_or_else(|| "missing source declaration".to_string())?;
    let sink = sink.ok_or_else(|| "missing sink declaration".to_string())?;
    if source == sink {
        return Err("source and sink must be distinct".to_string());
    }

    let nodes = (1..=node_count)
        .map(|id| FlowNode::new(format!("node-{id:010}")))
        .collect::<Vec<_>>();
    // Arc line order is preserved as identity. Parallel arcs are not merged.
    let edges = arcs
        .into_iter()
        .enumerate()
        .map(|(index, (from, to, capacity, line_number))| {
            FlowEdge::new(
                format!("arc-{index:010}"),
                format!("node-{from:010}"),
                format!("node-{to:010}"),
                capacity,
                Some(format!("dimacs-max-v1/line/{line_number}")),
            )
        })
        .collect::<Vec<_>>();

    Ok(FlowGraph {
        commodity_ref: "dimacs-single-commodity".to_string(),
        capacity_unit_ref: "dimacs-integer-capacity-unit".to_string(),
        quantity_boundary_ref: "dimacs-max-input-v1".to_string(),
        nodes,
        edges,
        source: format!("node-{source:010}"),
        sink: format!("node-{sink:010}"),
    })
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let path = args
        .next()
        .ok_or_else(|| "usage: cargo run --example dimacs_max -- <instance.max>".to_string())?;
    if args.next().is_some() {
        return Err("exactly one DIMACS input path is required".to_string());
    }

    let input = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {path}: {error}"))?;
    let graph = parse_dimacs_max(&input)?;
    let result = graph.solve().map_err(|error| error.to_string())?;
    let bundle = FlowCertificateBundle::new(graph, result)
        .map_err(|error| error.to_string())?;
    bundle.verify().map_err(|error| error.to_string())?;

    let stdout = io::stdout();
    let mut output = stdout.lock();
    serde_json::to_writer_pretty(&mut output, &bundle)
        .map_err(|error| format!("cannot serialize certificate bundle: {error}"))?;
    writeln!(output).map_err(|error| error.to_string())?;
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("DIMACS max-flow import failed: {error}");
        process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TWO_PATHS: &str = "c two path fixture
p max 4 5
n 1 s
n 4 t
a 1 2 3
a 1 3 2
a 2 4 2
a 3 4 2
a 2 3 1
";

    #[test]
    fn parses_standard_dimacs_and_emits_a_valid_certificate() {
        let graph = parse_dimacs_max(TWO_PATHS).unwrap();
        assert_eq!(graph.nodes.len(), 4);
        assert_eq!(graph.edges.len(), 5);
        let result = graph.solve().unwrap();
        assert_eq!(result.max_flow, 4);
        graph.verify_result(&result).unwrap();
        let bundle = FlowCertificateBundle::new(graph, result).unwrap();
        bundle.verify().unwrap();
    }

    #[test]
    fn parallel_arcs_keep_distinct_stable_identities() {
        let input = "p max 2 2
n 1 s
n 2 t
a 1 2 1
a 1 2 2
";
        let graph = parse_dimacs_max(input).unwrap();
        assert_eq!(graph.edges.len(), 2);
        assert_ne!(graph.edges[0].id, graph.edges[1].id);
        let result = graph.solve().unwrap();
        assert_eq!(result.max_flow, 3);
        assert_eq!(result.edge_flows.len(), 2);
        graph.verify_result(&result).unwrap();
    }

    #[test]
    fn rejects_capacity_above_dimacs_format_range() {
        let input = format!(
            "p max 2 1\nn 1 s\nn 2 t\na 1 2 {}\n",
            DIMACS_MAX_CAPACITY + 1
        );
        assert!(parse_dimacs_max(&input).unwrap_err().contains("DIMACS .max capacity"));
    }

    #[test]
    fn rejects_declared_arc_count_mismatch_and_out_of_range_nodes() {
        let short = "p max 2 2\nn 1 s\nn 2 t\na 1 2 1\n";
        assert!(parse_dimacs_max(short).unwrap_err().contains("declared 2 arcs"));

        let bad_node = "p max 2 1\nn 1 s\nn 2 t\na 1 3 1\n";
        assert!(parse_dimacs_max(bad_node).unwrap_err().contains("arc endpoint"));
    }

    #[test]
    fn rejects_oversized_instances_before_building_the_graph() {
        let oversized = format!(
            "p max {} 0\nn 1 s\nn 2 t\n",
            MAX_NODES + 1
        );
        assert!(parse_dimacs_max(&oversized).unwrap_err().contains("node count outside supported"));
    }
}

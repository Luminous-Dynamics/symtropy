use serde::Serialize;
use sha2::{Digest, Sha256};

pub const DESIGN_SKELETON_SCHEMA: &str = "pcb-design-skeleton-fingerprint-v0.1";
pub const DESIGN_SKELETON_COMPARISON_SCHEMA: &str = "pcb-design-skeleton-comparison-v0.1";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DesignSkeletonFingerprint {
    pub schema: String,
    pub board_sha256: String,
    pub design_skeleton_sha256: String,
    pub retained_root_node_count: usize,
    pub excluded_segments: usize,
    pub excluded_vias: usize,
    pub excluded_routed_arcs: usize,
    pub excluded_zone_fill_polygons: usize,
    pub authority: String,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DesignSkeletonComparison {
    pub schema: String,
    pub input: DesignSkeletonFingerprint,
    pub output: DesignSkeletonFingerprint,
    pub skeleton_matches: bool,
    pub authority: String,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Sexp {
    Atom(String),
    List(Vec<Sexp>),
}

impl Sexp {
    fn head(&self) -> Option<&str> {
        match self {
            Self::List(items) => items.first().and_then(|item| match item {
                Self::Atom(value) => Some(value.as_str()),
                Self::List(_) => None,
            }),
            Self::Atom(_) => None,
        }
    }

    fn has_direct_attribute(&self, name: &str) -> bool {
        match self {
            Self::List(items) => items.iter().skip(1).any(|item| item.head() == Some(name)),
            Self::Atom(_) => false,
        }
    }

    fn canonical(&self, output: &mut String) {
        match self {
            Self::Atom(value) => output.push_str(value),
            Self::List(items) => {
                output.push('(');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        output.push(' ');
                    }
                    item.canonical(output);
                }
                output.push(')');
            }
        }
    }
}

struct PruneCounts {
    segments: usize,
    vias: usize,
    arcs: usize,
    zone_fill_polygons: usize,
}

pub fn fingerprint_board_bytes(board_bytes: &[u8]) -> Result<DesignSkeletonFingerprint, String> {
    let text = std::str::from_utf8(board_bytes)
        .map_err(|error| format!("KiCad board is not UTF-8: {error}"))?;
    let tokens = tokenize(text)?;
    let mut cursor = 0usize;
    let parsed = parse_one(&tokens, &mut cursor)?;
    if cursor != tokens.len() {
        return Err("KiCad board has trailing S-expression tokens".into());
    }

    let mut root = match parsed {
        Sexp::List(items) => items,
        Sexp::Atom(_) => return Err("KiCad board root must be an S-expression list".into()),
    };
    if !matches!(root.first(), Some(Sexp::Atom(value)) if value == "kicad_pcb") {
        return Err("KiCad board root must be kicad_pcb".into());
    }

    let mut counts = PruneCounts {
        segments: 0,
        vias: 0,
        arcs: 0,
        zone_fill_polygons: 0,
    };
    let mut retained = Vec::with_capacity(root.len());
    for (index, node) in root.into_iter().enumerate() {
        if index == 0 {
            retained.push(node);
            continue;
        }
        match node.head() {
            Some("segment") => {
                require_attributes(&node, &["start", "end", "width", "layer", "net"], "track segment")?;
                counts.segments += 1;
            }
            Some("via") => {
                require_attributes(&node, &["at", "size", "drill", "layers", "net"], "via")?;
                counts.vias += 1;
            }
            Some("arc") if node.has_direct_attribute("net") => {
                require_attributes(&node, &["start", "mid", "end", "width", "layer", "net"], "routed arc")?;
                counts.arcs += 1;
            }
            Some("zone") => retained.push(prune_zone_fill(node, &mut counts)),
            _ => retained.push(node),
        }
    }

    let canonical = Sexp::List(retained.clone());
    let mut skeleton_bytes = String::new();
    canonical.canonical(&mut skeleton_bytes);

    Ok(DesignSkeletonFingerprint {
        schema: DESIGN_SKELETON_SCHEMA.into(),
        board_sha256: sha256_hex(board_bytes),
        design_skeleton_sha256: sha256_hex(skeleton_bytes.as_bytes()),
        retained_root_node_count: retained.len(),
        excluded_segments: counts.segments,
        excluded_vias: counts.vias,
        excluded_routed_arcs: counts.arcs,
        excluded_zone_fill_polygons: counts.zone_fill_polygons,
        authority: "native_kicad_sexpression_design_structure_comparison_only".into(),
        claim_ceiling: "The fingerprint ignores recognized root-level copper segments, vias, net-assigned routed arcs, and generated zone-fill polygons. It preserves other parsed design structure, but is not a complete semantic equivalence proof, DRC pass, signal/power-integrity signoff, physical qualification, or production release.".into(),
    })
}

pub fn compare_board_design_skeletons(
    input_board_bytes: &[u8],
    output_board_bytes: &[u8],
) -> Result<DesignSkeletonComparison, String> {
    let input = fingerprint_board_bytes(input_board_bytes)?;
    let output = fingerprint_board_bytes(output_board_bytes)?;
    Ok(DesignSkeletonComparison {
        schema: DESIGN_SKELETON_COMPARISON_SCHEMA.into(),
        skeleton_matches: input.design_skeleton_sha256 == output.design_skeleton_sha256,
        input,
        output,
        authority: "input_output_native_board_skeleton_hash_comparison_only".into(),
        claim_ceiling: "A matching fingerprint is a conservative structural guard, not full KiCad semantic equivalence. A mismatch must be investigated and must not count as an unchanged-design routing success.".into(),
    })
}

fn prune_zone_fill(zone: Sexp, counts: &mut PruneCounts) -> Sexp {
    match zone {
        Sexp::List(items) => {
            let mut retained = Vec::with_capacity(items.len());
            for (index, item) in items.into_iter().enumerate() {
                if index > 0 && item.head() == Some("filled_polygon") {
                    counts.zone_fill_polygons += 1;
                } else {
                    retained.push(item);
                }
            }
            Sexp::List(retained)
        }
        Sexp::Atom(_) => zone,
    }
}

fn require_attributes(node: &Sexp, names: &[&str], context: &str) -> Result<(), String> {
    if names.iter().any(|name| !node.has_direct_attribute(name)) {
        return Err(format!(
            "unrecognized or incomplete {context} node cannot be safely excluded from design skeleton"
        ));
    }
    Ok(())
}

fn tokenize(input: &str) -> Result<Vec<String>, String> {
    let bytes = input.as_bytes();
    let mut tokens = Vec::new();
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        match bytes[cursor] {
            byte if byte.is_ascii_whitespace() => cursor += 1,
            b'(' => {
                tokens.push("(".into());
                cursor += 1;
            }
            b')' => {
                tokens.push(")".into());
                cursor += 1;
            }
            b'"' => {
                let start = cursor;
                cursor += 1;
                let mut closed = false;
                while cursor < bytes.len() {
                    match bytes[cursor] {
                        b'\\' => {
                            cursor += 1;
                            if cursor >= bytes.len() {
                                return Err("unterminated escape in quoted S-expression atom".into());
                            }
                            cursor += 1;
                        }
                        b'"' => {
                            cursor += 1;
                            closed = true;
                            break;
                        }
                        _ => cursor += 1,
                    }
                }
                if !closed {
                    return Err("unterminated quoted S-expression atom".into());
                }
                tokens.push(input[start..cursor].to_string());
            }
            _ => {
                let start = cursor;
                while cursor < bytes.len()
                    && !bytes[cursor].is_ascii_whitespace()
                    && bytes[cursor] != b'('
                    && bytes[cursor] != b')'
                {
                    cursor += 1;
                }
                tokens.push(input[start..cursor].to_string());
            }
        }
    }
    Ok(tokens)
}

fn parse_one(tokens: &[String], cursor: &mut usize) -> Result<Sexp, String> {
    let token = tokens
        .get(*cursor)
        .ok_or_else(|| "unexpected end of S-expression".to_string())?;
    if token == ")" {
        return Err("unexpected closing parenthesis in S-expression".into());
    }
    if token != "(" {
        *cursor += 1;
        return Ok(Sexp::Atom(token.clone()));
    }

    *cursor += 1;
    let mut items = Vec::new();
    loop {
        let token = tokens
            .get(*cursor)
            .ok_or_else(|| "unclosed parenthesis in S-expression".to_string())?;
        if token == ")" {
            *cursor += 1;
            break;
        }
        items.push(parse_one(tokens, cursor)?);
    }
    Ok(Sexp::List(items))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOARD: &str = r#"(kicad_pcb
      (version 20241229)
      (generator "pcbnew")
      (general (thickness 1.6))
      (layers (0 "F.Cu" signal) (31 "F.CrtYd" user) (25 "Edge.Cuts" user))
      (setup (pad_to_mask_clearance 0)
        (net_class "Default" "" (clearance 0.2) (trace_width 0.5))
      )
      (net 0 "") (net 1 "VCC") (net 2 "GND")
      (footprint "AtomFab:Connector"
        (layer "F.Cu") (at 10 10)
        (property "Reference" "J1")
        (pad "1" thru_hole rect (at 0 0) (size 1.7 1.7) (drill 1)
          (layers "*.Cu" "*.Mask") (net 1 "VCC"))
        (pad "2" thru_hole circle (at 0 2.54) (size 1.7 1.7) (drill 1)
          (layers "*.Cu" "*.Mask") (net 2 "GND"))
      )
      (gr_rect (start 0 0) (end 20 10) (layer "Edge.Cuts"))
      (zone (net 2) (layer "B.Cu") (min_thickness 0.25)
        (polygon (pts (xy 0 0) (xy 20 0) (xy 20 10) (xy 0 10)))
        (filled_polygon (layer "B.Cu") (pts (xy 1 1) (xy 19 1)))
      )
    )"#;

    fn routed_board() -> String {
        BOARD
            .replace(
                "(zone (net 2)",
                r#"(segment (start 10 10) (end 12 10) (width 0.5) (layer "F.Cu") (net 1))
              (via (at 12 10) (size 0.8) (drill 0.4) (layers "F.Cu" "B.Cu") (net 1))
              (arc (start 12 10) (mid 13 11) (end 14 10) (width 0.5) (layer "F.Cu") (net 1))
              (zone (net 2)"#,
            )
            .replace("(xy 1 1) (xy 19 1)", "(xy 2 2) (xy 18 2)")
    }

    #[test]
    fn routing_primitives_and_zone_refill_do_not_change_design_skeleton() {
        let comparison =
            compare_board_design_skeletons(BOARD.as_bytes(), routed_board().as_bytes()).unwrap();
        assert!(comparison.skeleton_matches);
        assert_eq!(comparison.input.excluded_segments, 0);
        assert_eq!(comparison.output.excluded_segments, 1);
        assert_eq!(comparison.output.excluded_vias, 1);
        assert_eq!(comparison.output.excluded_routed_arcs, 1);
        assert_eq!(comparison.output.excluded_zone_fill_polygons, 1);
    }

    #[test]
    fn placement_net_rule_and_outline_changes_are_not_routing() {
        let placement = BOARD.replace("(at 10 10)", "(at 10.25 10)");
        let net_assignment = BOARD.replace("(net 1 \\"VCC\\")", "(net 2 \\"VCC\\")");
        let clearance = BOARD.replace("(clearance 0.2)", "(clearance 0.25)");
        let outline = BOARD.replace("(end 20 10)", "(end 21 10)");
        for changed in [placement, net_assignment, clearance, outline] {
            let comparison = compare_board_design_skeletons(BOARD.as_bytes(), changed.as_bytes()).unwrap();
            assert!(!comparison.skeleton_matches);
        }
    }

    #[test]
    fn malformed_route_primitives_fail_closed_instead_of_being_stripped() {
        let malformed = BOARD.replace(
            "(zone (net 2)",
            "(segment (start 10 10) (end 12 10) (layer \\"F.Cu\\") (net 1)) (zone (net 2)",
        );
        assert!(fingerprint_board_bytes(malformed.as_bytes())
            .unwrap_err()
            .contains("unrecognized or incomplete track segment"));
    }

    #[test]
    fn malformed_and_non_board_sexpressions_reject() {
        for value in [
            "(kicad_pcb (version 20241229)",
            "(not_a_board)",
            "(kicad_pcb) garbage",
            "(kicad_pcb ) )",
            "(kicad_pcb (comment \\"unterminated))",
        ] {
            assert!(fingerprint_board_bytes(value.as_bytes()).is_err(), "{value}");
        }
    }
}

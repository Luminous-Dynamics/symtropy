//! Independent semantic oracle for Symtropy furnishing-layout fixtures.
//!
//! This crate audits known-truth synthetic geometry; it does not establish
//! building-code compliance, physical safety, measured-site truth, or usability.

pub mod alternative_frontier;
pub mod resident_preference_probe;

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

fn field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value.as_object()?.get(key)
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    field(value, key)?.as_str()
}

fn array<'a>(value: &'a Value, key: &str) -> Option<&'a Vec<Value>> {
    field(value, key)?.as_array()
}

fn positive_finite(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_f64)
        .is_some_and(|number| number.is_finite() && number > 0.0)
}

#[derive(Clone, Copy, Debug)]
struct Rect {
    x: f64,
    y: f64,
    width: f64,
    depth: f64,
}

impl Rect {
    fn from_value(value: &Value) -> Option<Self> {
        Some(Self {
            x: field(value, "x_m")?.as_f64()?,
            y: field(value, "y_m")?.as_f64()?,
            width: field(value, "width_m")?.as_f64()?,
            depth: field(value, "depth_m")?.as_f64()?,
        })
    }

    fn is_valid(self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.width.is_finite()
            && self.depth.is_finite()
            && self.x >= 0.0
            && self.y >= 0.0
            && self.width > 0.0
            && self.depth > 0.0
    }

    fn inside_room(self, width: f64, depth: f64) -> bool {
        self.x + self.width <= width + 1e-9 && self.y + self.depth <= depth + 1e-9
    }

    fn overlaps(self, other: Self) -> bool {
        self.x < other.x + other.width - 1e-9
            && self.x + self.width > other.x + 1e-9
            && self.y < other.y + other.depth - 1e-9
            && self.y + self.depth > other.y + 1e-9
    }
}

#[derive(Clone, Debug)]
struct SpatialEntry {
    item_id: String,
    rect: Rect,
}

#[derive(Clone, Debug)]
enum SpatialNodeKind {
    Leaf(Vec<usize>),
    Branch {
        left: Box<SpatialNode>,
        right: Box<SpatialNode>,
    },
}

#[derive(Clone, Debug)]
struct SpatialNode {
    bounds: Rect,
    kind: SpatialNodeKind,
}

/// Immutable bounding-volume hierarchy used only as a broad phase. Exact
/// rectangle intersection remains the narrow-phase predicate; leaf traversal
/// cannot convert an approximation into a physical pass.
#[derive(Clone, Debug)]
struct SpatialIndex {
    entries: Vec<SpatialEntry>,
    root: Option<Box<SpatialNode>>,
}

impl SpatialIndex {
    const LEAF_CAPACITY: usize = 8;

    fn new(items: &[(String, Rect)]) -> Self {
        let entries = items
            .iter()
            .map(|(item_id, rect)| SpatialEntry {
                item_id: item_id.clone(),
                rect: *rect,
            })
            .collect::<Vec<_>>();
        let indices = (0..entries.len()).collect::<Vec<_>>();
        let root = Self::build_node(indices, &entries);
        Self { entries, root }
    }

    fn build_node(indices: Vec<usize>, entries: &[SpatialEntry]) -> Option<Box<SpatialNode>> {
        let first = *indices.first()?;
        let bounds = indices
            .iter()
            .skip(1)
            .fold(entries[first].rect, |current, index| {
                union_rect(current, entries[*index].rect)
            });

        if indices.len() <= Self::LEAF_CAPACITY {
            return Some(Box::new(SpatialNode {
                bounds,
                kind: SpatialNodeKind::Leaf(indices),
            }));
        }

        let split_x = bounds.width >= bounds.depth;
        let mut sorted = indices;
        sorted.sort_by(|left, right| {
            let left_rect = entries[*left].rect;
            let right_rect = entries[*right].rect;
            let left_center = if split_x {
                left_rect.x + left_rect.width * 0.5
            } else {
                left_rect.y + left_rect.depth * 0.5
            };
            let right_center = if split_x {
                right_rect.x + right_rect.width * 0.5
            } else {
                right_rect.y + right_rect.depth * 0.5
            };
            left_center
                .total_cmp(&right_center)
                .then_with(|| left.cmp(right))
        });
        let right_indices = sorted.split_off(sorted.len() / 2);
        let left = Self::build_node(sorted, entries)?;
        let right = Self::build_node(right_indices, entries)?;
        Some(Box::new(SpatialNode {
            bounds,
            kind: SpatialNodeKind::Branch { left, right },
        }))
    }

    fn query_indices(&self, query: Rect) -> Vec<usize> {
        let mut matches = Vec::new();
        if let Some(root) = &self.root {
            Self::query_node(root, query, &self.entries, &mut matches);
        }
        matches
    }

    fn query_node(
        node: &SpatialNode,
        query: Rect,
        entries: &[SpatialEntry],
        matches: &mut Vec<usize>,
    ) {
        if !node.bounds.overlaps(query) {
            return;
        }
        match &node.kind {
            SpatialNodeKind::Leaf(indices) => {
                for index in indices {
                    if entries[*index].rect.overlaps(query) {
                        matches.push(*index);
                    }
                }
            }
            SpatialNodeKind::Branch { left, right } => {
                Self::query_node(left, query, entries, matches);
                Self::query_node(right, query, entries, matches);
            }
        }
    }

    /// Returns each overlapping pair once, ordered by original input index.
    fn overlapping_pairs(&self) -> Vec<(usize, usize)> {
        let mut pairs = Vec::new();
        for left in 0..self.entries.len() {
            let mut candidates = self.query_indices(self.entries[left].rect);
            candidates.sort_unstable();
            for right in candidates {
                if right > left {
                    pairs.push((left, right));
                }
            }
        }
        pairs
    }
}

fn union_rect(left: Rect, right: Rect) -> Rect {
    let min_x = left.x.min(right.x);
    let min_y = left.y.min(right.y);
    let max_x = (left.x + left.width).max(right.x + right.width);
    let max_y = (left.y + left.depth).max(right.y + right.depth);
    Rect {
        x: min_x,
        y: min_y,
        width: max_x - min_x,
        depth: max_y - min_y,
    }
}

#[derive(Clone, Copy, Debug)]
struct ItemSize {
    width: f64,
    depth: f64,
}

struct EnvelopeDefinition<'a> {
    envelope: &'a Value,
    owner_id: String,
    owner_size: ItemSize,
}

fn unique_ids(
    items: &[Value],
    key: &str,
    label: &str,
    errors: &mut Vec<String>,
) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for (index, item) in items.iter().enumerate() {
        let Some(id) = text(item, key) else {
            errors.push(format!("{label}[{index}]: missing string {key}"));
            continue;
        };
        if id.trim().is_empty() {
            errors.push(format!("{label}[{index}]: blank {key}"));
        } else if !ids.insert(id.to_owned()) {
            errors.push(format!("{label}: duplicate {key} {id:?}"));
        }
    }
    ids
}

fn placement_rect(placement: &Value, sizes: &BTreeMap<String, ItemSize>) -> Option<(String, Rect)> {
    let item_id = text(placement, "item_id")?.to_owned();
    let item = sizes.get(&item_id)?;
    let x = field(placement, "x_m")?.as_f64()?;
    let y = field(placement, "y_m")?.as_f64()?;
    let rotation = field(placement, "rotation_degrees")?.as_u64()?;
    let (width, depth) = match rotation {
        0 | 180 => (item.width, item.depth),
        90 | 270 => (item.depth, item.width),
        _ => return None,
    };
    Some((item_id, Rect { x, y, width, depth }))
}

fn requirement_passes_zone_clearance(
    scenario: &Value,
    requirement: &Value,
    placements: &[Value],
    sizes: &BTreeMap<String, ItemSize>,
) -> Option<bool> {
    let zone_id = text(requirement, "target_zone_id")?;
    let zone = array(scenario, "zones")?
        .iter()
        .find(|zone| text(zone, "zone_id") == Some(zone_id))?;
    let zone_rect = Rect::from_value(field(zone, "rect")?)?;
    let mut collision = false;
    for placement in placements {
        let (_, item_rect) = placement_rect(placement, sizes)?;
        if item_rect.overlaps(zone_rect) {
            collision = true;
        }
    }
    Some(!collision)
}

fn actual_expected_outcome(
    scenario: &Value,
    requirement: &Value,
    placements: &[Value],
    sizes: &BTreeMap<String, ItemSize>,
) -> Option<&'static str> {
    match text(requirement, "kind")? {
        "zone_must_remain_clear" | "item_must_avoid_zone" => {
            if requirement_passes_zone_clearance(scenario, requirement, placements, sizes)? {
                Some("pass")
            } else {
                Some("fail")
            }
        }
        "required_geometry_evidence" => {
            if text(field(scenario, "room")?, "geometry_evidence_state")? == "unknown_for_real_site"
            {
                Some("indeterminate")
            } else {
                Some("pass")
            }
        }
        // Preference outcomes are synthetic judgments supplied by the
        // fixture. They are deliberately not converted into physical facts.
        "preference_tradeoff" => None,
        _ => None,
    }
}

fn preference_relation(left: &Value, right: &Value) -> Option<&'static str> {
    let left_values = array(left, "preference_observations")?;
    let right_values = array(right, "preference_observations")?;
    if left_values.is_empty() || right_values.is_empty() {
        return None;
    }

    let to_map = |values: &[Value]| -> Option<BTreeMap<String, i64>> {
        let mut result = BTreeMap::new();
        for entry in values {
            let dimension = text(entry, "dimension")?.to_owned();
            let rating = field(entry, "value_0_to_100")?.as_i64()?;
            if !(0..=100).contains(&rating) || result.insert(dimension, rating).is_some() {
                return None;
            }
        }
        Some(result)
    };
    let left_map = to_map(left_values)?;
    let right_map = to_map(right_values)?;
    if left_map.keys().collect::<Vec<_>>() != right_map.keys().collect::<Vec<_>>() {
        return None;
    }

    let left_at_least = left_map.iter().all(|(key, value)| value >= &right_map[key]);
    let right_at_least = right_map.iter().all(|(key, value)| value >= &left_map[key]);
    let left_strict = left_map.iter().any(|(key, value)| value > &right_map[key]);
    let right_strict = right_map.iter().any(|(key, value)| value > &left_map[key]);

    match (left_at_least, right_at_least, left_strict, right_strict) {
        (true, false, true, false) => Some("dominates"),
        (false, true, false, true) => Some("dominated"),
        (true, true, false, false) => Some("equal"),
        _ => Some("incomparable"),
    }
}

fn validate_scenario(scenario: &Value, label: &str, errors: &mut Vec<String>) {
    if text(scenario, "scope") != Some("synthetic_only") {
        errors.push(format!(
            "{label}: non-synthetic scope is outside this auditor's claim boundary"
        ));
    }

    let Some(room) = field(scenario, "room") else {
        errors.push(format!("{label}: missing room"));
        return;
    };
    let Some(room_width) = field(room, "width_m").and_then(Value::as_f64) else {
        errors.push(format!("{label}: room.width_m must be numeric"));
        return;
    };
    let Some(room_depth) = field(room, "depth_m").and_then(Value::as_f64) else {
        errors.push(format!("{label}: room.depth_m must be numeric"));
        return;
    };
    if !room_width.is_finite() || room_width <= 0.0 || !room_depth.is_finite() || room_depth <= 0.0
    {
        errors.push(format!(
            "{label}: room dimensions must be positive and finite"
        ));
    }

    let Some(catalog) = array(scenario, "catalog") else {
        errors.push(format!("{label}: missing catalog"));
        return;
    };
    let catalog_ids = unique_ids(catalog, "item_id", &format!("{label}.catalog"), errors);
    let mut sizes = BTreeMap::new();
    for item in catalog {
        let Some(id) = text(item, "item_id") else {
            continue;
        };
        let width = field(item, "width_m").and_then(Value::as_f64);
        let depth = field(item, "depth_m").and_then(Value::as_f64);
        if !positive_finite(field(item, "width_m")) || !positive_finite(field(item, "depth_m")) {
            errors.push(format!(
                "{label}.catalog.{id}: width/depth must be positive and finite"
            ));
            continue;
        }
        if let (Some(width), Some(depth)) = (width, depth) {
            sizes.insert(id.to_owned(), ItemSize { width, depth });
        }
    }

    let Some(zones) = array(scenario, "zones") else {
        errors.push(format!("{label}: missing zones"));
        return;
    };
    let zone_ids = unique_ids(zones, "zone_id", &format!("{label}.zones"), errors);
    for zone in zones {
        let Some(zone_id) = text(zone, "zone_id") else {
            continue;
        };
        let Some(rect_value) = field(zone, "rect") else {
            errors.push(format!("{label}.zones.{zone_id}: missing rect"));
            continue;
        };
        let Some(rect) = Rect::from_value(rect_value) else {
            errors.push(format!("{label}.zones.{zone_id}: malformed rect"));
            continue;
        };
        if !rect.is_valid() || !rect.inside_room(room_width, room_depth) {
            errors.push(format!(
                "{label}.zones.{zone_id}: rectangle is invalid or outside nominal room"
            ));
        }
    }

    let Some(requirements) = array(scenario, "requirements") else {
        errors.push(format!("{label}: missing requirements"));
        return;
    };
    let requirement_ids = unique_ids(
        requirements,
        "requirement_id",
        &format!("{label}.requirements"),
        errors,
    );
    for requirement in requirements {
        let Some(requirement_id) = text(requirement, "requirement_id") else {
            continue;
        };
        if let Some(target) = text(requirement, "target_zone_id")
            && !zone_ids.contains(target)
        {
            errors.push(format!(
                "{label}.requirements.{requirement_id}: unknown target zone {target:?}"
            ));
        }
        if let Some(target) = text(requirement, "target_item_id")
            && !catalog_ids.contains(target)
        {
            errors.push(format!(
                "{label}.requirements.{requirement_id}: unknown target item {target:?}"
            ));
        }
        if text(requirement, "classification") == Some("unknown_requires_review")
            && text(requirement, "evidence_state") != Some("unknown")
        {
            errors.push(format!("{label}.requirements.{requirement_id}: unknown requirement cannot claim known evidence"));
        }
    }

    let Some(alternatives) = array(scenario, "alternatives") else {
        errors.push(format!("{label}: missing alternatives"));
        return;
    };
    if alternatives.is_empty() {
        errors.push(format!("{label}: alternatives must not be empty"));
    }
    unique_ids(
        alternatives,
        "alternative_id",
        &format!("{label}.alternatives"),
        errors,
    );

    let mut any_unresolved_mandatory = false;
    for alternative in alternatives {
        let Some(alternative_id) = text(alternative, "alternative_id") else {
            continue;
        };
        let Some(placements) = array(alternative, "placements") else {
            errors.push(format!("{label}.{alternative_id}: missing placements"));
            continue;
        };
        let placement_ids = unique_ids(
            placements,
            "item_id",
            &format!("{label}.{alternative_id}.placements"),
            errors,
        );
        for item_id in &placement_ids {
            if !catalog_ids.contains(item_id) {
                errors.push(format!(
                    "{label}.{alternative_id}: placement references unknown item {item_id:?}"
                ));
            }
        }

        let mut geometry = Vec::new();
        for placement in placements {
            let Some(item_id) = text(placement, "item_id") else {
                continue;
            };
            let Some(rotation) = field(placement, "rotation_degrees").and_then(Value::as_u64)
            else {
                errors.push(format!(
                    "{label}.{alternative_id}.{item_id}: missing numeric rotation_degrees"
                ));
                continue;
            };
            if ![0, 90, 180, 270].contains(&rotation) {
                errors.push(format!(
                    "{label}.{alternative_id}.{item_id}: unsupported rotation {rotation}"
                ));
                continue;
            }
            let Some((_, rect)) = placement_rect(placement, &sizes) else {
                errors.push(format!(
                    "{label}.{alternative_id}.{item_id}: cannot derive footprint"
                ));
                continue;
            };
            if !rect.is_valid() || !rect.inside_room(room_width, room_depth) {
                errors.push(format!("{label}.{alternative_id}.{item_id}: footprint is invalid or outside nominal room"));
            }
            geometry.push((item_id.to_owned(), rect));
        }

        for left in 0..geometry.len() {
            for right in (left + 1)..geometry.len() {
                if geometry[left].1.overlaps(geometry[right].1) {
                    errors.push(format!(
                        "{label}.{alternative_id}: furniture {} overlaps {}",
                        geometry[left].0, geometry[right].0
                    ));
                }
            }
        }

        let Some(checks) = array(alternative, "expected_checks") else {
            errors.push(format!("{label}.{alternative_id}: missing expected_checks"));
            continue;
        };
        let check_ids = unique_ids(
            checks,
            "requirement_id",
            &format!("{label}.{alternative_id}.expected_checks"),
            errors,
        );
        for check in checks {
            let Some(requirement_id) = text(check, "requirement_id") else {
                continue;
            };
            if !requirement_ids.contains(requirement_id) {
                errors.push(format!("{label}.{alternative_id}: check references unknown requirement {requirement_id:?}"));
                continue;
            }
            let Some(requirement) = requirements
                .iter()
                .find(|requirement| text(requirement, "requirement_id") == Some(requirement_id))
            else {
                continue;
            };
            if let Some(expected) =
                actual_expected_outcome(scenario, requirement, placements, &sizes)
                && text(check, "outcome") != Some(expected)
            {
                errors.push(format!(
                    "{label}.{alternative_id}.{requirement_id}: expected outcome should be {expected:?}, got {:?}",
                    text(check, "outcome")
                ));
            }
            if text(requirement, "classification") == Some("unknown_requires_review")
                && text(check, "outcome") == Some("indeterminate")
                && field(requirement, "must_be_satisfied_to_advance").and_then(Value::as_bool)
                    == Some(true)
            {
                any_unresolved_mandatory = true;
            }
        }

        // Missing or indeterminate mandatory requirements cannot be promoted
        // as a technically assessed layout.
        let hard_failure = checks.iter().any(|check| {
            let requirement_id = text(check, "requirement_id");
            let requirement = requirements
                .iter()
                .find(|requirement| text(requirement, "requirement_id") == requirement_id);
            requirement.is_some_and(|requirement| {
                field(requirement, "must_be_satisfied_to_advance").and_then(Value::as_bool)
                    == Some(true)
                    && matches!(text(check, "outcome"), Some("fail" | "indeterminate"))
            })
        });
        if hard_failure && text(alternative, "promotion_disposition") != Some("blocked") {
            errors.push(format!("{label}.{alternative_id}: unresolved/failing mandatory requirement must remain blocked"));
        }
        if !hard_failure && text(alternative, "promotion_disposition") == Some("blocked") {
            // Explicitly blocked alternatives are allowed but are not silently
            // treated as passing; no assumption is made about their disposition.
        }

        if let Some(expected_relation) = text(alternative, "expected_pareto_relation") {
            let peers = alternatives.iter().filter(|peer| {
                text(peer, "alternative_id") != Some(alternative_id)
                    && text(peer, "expected_pareto_relation").is_some()
            });
            for peer in peers {
                if let Some(relation) = preference_relation(alternative, peer)
                    && expected_relation != relation
                {
                    errors.push(format!(
                        "{label}.{alternative_id}: Pareto oracle mismatch; expected field {expected_relation:?}, calculated {relation:?}"
                    ));
                }
            }
        }

        // Every placed object identity may appear at most once. Keep this
        // separate from geometric overlap so diagnostics identify bad inputs.
        if placement_ids.len() != placements.len() {
            errors.push(format!(
                "{label}.{alternative_id}: duplicate item placement"
            ));
        }
        if check_ids.len() != checks.len() {
            errors.push(format!(
                "{label}.{alternative_id}: duplicate requirement check"
            ));
        }
    }

    if any_unresolved_mandatory
        && alternatives
            .iter()
            .any(|alternative| text(alternative, "promotion_disposition") != Some("blocked"))
    {
        errors.push(format!(
            "{label}: unknown required site evidence must block every alternative"
        ));
    }
}

pub fn audit_fixtures(document: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    if text(document, "schema_version") != Some("0.1.0") {
        errors.push("root: unsupported schema_version".to_owned());
    }
    if text(document, "benchmark_id") != Some("furnishing-layout-known-truth-001") {
        errors.push("root: unexpected benchmark_id".to_owned());
    }
    if text(document, "status") != Some("synthetic_specification_not_run") {
        errors.push("root: fixtures must remain explicitly marked not run".to_owned());
    }

    let Some(frame) = field(document, "coordinate_frame") else {
        errors.push("root: missing coordinate_frame".to_owned());
        return errors;
    };
    let rotations: BTreeSet<u64> = array(frame, "rotation_degrees_allowed")
        .into_iter()
        .flatten()
        .filter_map(Value::as_u64)
        .collect();
    if rotations != BTreeSet::from([0, 90, 180, 270]) {
        errors.push("root.coordinate_frame: rotations must be exactly 0, 90, 180, 270".to_owned());
    }
    if text(frame, "geometry_model") != Some("axis_aligned_2d_rectangles") {
        errors.push("root.coordinate_frame: unsupported geometry model".to_owned());
    }
    if text(field(document, "units").unwrap_or(&Value::Null), "length") != Some("m")
        || text(field(document, "units").unwrap_or(&Value::Null), "area") != Some("m2")
    {
        errors.push("root.units: this fixture audit requires metres and square metres".to_owned());
    }

    let Some(scenarios) = array(document, "scenarios") else {
        errors.push("root: missing scenarios array".to_owned());
        return errors;
    };
    if scenarios.is_empty() {
        errors.push("root.scenarios must not be empty".to_owned());
    }
    unique_ids(scenarios, "scenario_id", "root.scenarios", &mut errors);
    for (index, scenario) in scenarios.iter().enumerate() {
        let label = text(scenario, "scenario_id")
            .map(str::to_owned)
            .unwrap_or_else(|| format!("scenario[{index}]"));
        validate_scenario(scenario, &label, &mut errors);
    }
    errors
}

/// Dispatch the independent oracle by the frozen fixture schema version.
pub fn audit_document(document: &Value) -> Vec<String> {
    match text(document, "schema_version") {
        Some("0.1.0") => audit_fixtures(document),
        Some("0.2.0") => audit_functional_envelopes(document),
        _ => vec!["root: unsupported schema_version".to_owned()],
    }
}

/// The fixture's placement coordinates identify the lower-left corner of the
/// quarter-turn-aligned footprint. Envelope rectangles are item-local, with
/// their origin at the unrotated item's south-west corner, and rotate with it.
fn transformed_functional_envelope(
    envelope: &Value,
    item_size: ItemSize,
    placement: &Value,
) -> Option<Rect> {
    let local = Rect::from_value(field(envelope, "rect_local_item_frame")?)?;
    let x = field(placement, "x_m")?.as_f64()?;
    let y = field(placement, "y_m")?.as_f64()?;
    let rotation = field(placement, "rotation_degrees")?.as_u64()?;

    let (x, y, width, depth) = match rotation {
        0 => (x + local.x, y + local.y, local.width, local.depth),
        90 => (
            x + item_size.depth - (local.y + local.depth),
            y + local.x,
            local.depth,
            local.width,
        ),
        180 => (
            x + item_size.width - (local.x + local.width),
            y + item_size.depth - (local.y + local.depth),
            local.width,
            local.depth,
        ),
        270 => (
            x + local.y,
            y + item_size.width - (local.x + local.width),
            local.depth,
            local.width,
        ),
        _ => return None,
    };

    Some(Rect { x, y, width, depth })
}

/// Compute a fixture outcome from indexed evidence and exact geometric checks.
/// A BVH only reduces candidates; every candidate is tested with Rect::overlaps.
fn expected_functional_envelope_outcome(
    scenario: &Value,
    requirement: &Value,
    envelope_definitions: &BTreeMap<String, EnvelopeDefinition<'_>>,
    placements_by_id: &BTreeMap<String, &Value>,
    footprint_index: &SpatialIndex,
) -> Option<&'static str> {
    let envelope_id = text(requirement, "target_envelope_id")?;
    let definition = envelope_definitions.get(envelope_id)?;
    let envelope = definition.envelope;
    if text(envelope, "evidence_state") == Some("unknown") {
        return Some("indeterminate");
    }
    if text(envelope, "evidence_state") != Some("synthetic_known_truth") {
        return None;
    }
    let owner_placement = *placements_by_id.get(&definition.owner_id)?;
    let envelope_rect =
        transformed_functional_envelope(envelope, definition.owner_size, owner_placement)?;
    let room = field(scenario, "room")?;
    let room_width = field(room, "width_m")?.as_f64()?;
    let room_depth = field(room, "depth_m")?.as_f64()?;

    if !envelope_rect.is_valid() || !envelope_rect.inside_room(room_width, room_depth) {
        return Some("fail");
    }
    for candidate_index in footprint_index.query_indices(envelope_rect) {
        let candidate = footprint_index.entries.get(candidate_index)?;
        if candidate.item_id != definition.owner_id {
            return Some("fail");
        }
    }
    Some("pass")
}

/// Independent synthetic known-truth oracle for item-local, task-specific
/// functional envelopes. It checks those envelopes separately from physical
/// footprint collisions. The envelope model is axis-aligned and limited to
/// front-side rectangular task zones transformed by quarter-turn rotation.
/// It is not a human biomechanics, accessibility-code, or safety certification.
pub fn audit_functional_envelopes(document: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    if text(document, "schema_version") != Some("0.2.0") {
        errors.push("root: unsupported schema_version for functional-envelope audit".to_owned());
    }
    if text(document, "benchmark_id") != Some("furnishing-functional-envelope-known-truth-001") {
        errors.push("root: unexpected functional-envelope benchmark_id".to_owned());
    }
    if text(document, "status") != Some("synthetic_specification_not_run") {
        errors.push(
            "root: functional-envelope fixtures must remain explicitly marked not run".to_owned(),
        );
    }
    if text(field(document, "units").unwrap_or(&Value::Null), "length") != Some("m") {
        errors.push("root.units: functional-envelope audit requires metres".to_owned());
    }

    let Some(frame) = field(document, "coordinate_frame") else {
        errors.push("root: missing coordinate_frame".to_owned());
        return errors;
    };
    let rotations: BTreeSet<u64> = array(frame, "rotation_degrees_allowed")
        .into_iter()
        .flatten()
        .filter_map(Value::as_u64)
        .collect();
    if rotations != BTreeSet::from([0, 90, 180, 270]) {
        errors.push("root.coordinate_frame: rotations must be exactly 0, 90, 180, 270".to_owned());
    }
    if text(frame, "geometry_model") != Some("axis_aligned_2d_rectangles") {
        errors.push("root.coordinate_frame: unsupported geometry model".to_owned());
    }

    let Some(scenarios) = array(document, "scenarios") else {
        errors.push("root: missing scenarios array".to_owned());
        return errors;
    };
    if scenarios.is_empty() {
        errors.push("root.scenarios must not be empty".to_owned());
    }
    unique_ids(scenarios, "scenario_id", "root.scenarios", &mut errors);

    for (scenario_index, scenario) in scenarios.iter().enumerate() {
        let label = text(scenario, "scenario_id")
            .map(str::to_owned)
            .unwrap_or_else(|| format!("scenario[{scenario_index}]"));
        if text(scenario, "scope") != Some("synthetic_only") {
            errors.push(format!(
                "{label}: functional-envelope oracle accepts synthetic-only scope"
            ));
        }
        let Some(room) = field(scenario, "room") else {
            errors.push(format!("{label}: missing room"));
            continue;
        };
        let Some(room_width) = field(room, "width_m").and_then(Value::as_f64) else {
            errors.push(format!("{label}: room.width_m must be numeric"));
            continue;
        };
        let Some(room_depth) = field(room, "depth_m").and_then(Value::as_f64) else {
            errors.push(format!("{label}: room.depth_m must be numeric"));
            continue;
        };
        if !room_width.is_finite()
            || !room_depth.is_finite()
            || room_width <= 0.0
            || room_depth <= 0.0
        {
            errors.push(format!(
                "{label}: room dimensions must be positive and finite"
            ));
            continue;
        }

        let Some(catalog) = array(scenario, "catalog") else {
            errors.push(format!("{label}: missing catalog"));
            continue;
        };
        let catalog_ids = unique_ids(catalog, "item_id", &format!("{label}.catalog"), &mut errors);
        let mut sizes = BTreeMap::new();
        let mut envelope_ids = BTreeSet::new();
        let mut envelope_definitions: BTreeMap<String, EnvelopeDefinition<'_>> = BTreeMap::new();
        for item in catalog {
            let Some(item_id) = text(item, "item_id") else {
                continue;
            };
            let Some(width) = field(item, "width_m").and_then(Value::as_f64) else {
                errors.push(format!(
                    "{label}.catalog.{item_id}: width_m must be numeric"
                ));
                continue;
            };
            let Some(depth) = field(item, "depth_m").and_then(Value::as_f64) else {
                errors.push(format!(
                    "{label}.catalog.{item_id}: depth_m must be numeric"
                ));
                continue;
            };
            if !width.is_finite() || !depth.is_finite() || width <= 0.0 || depth <= 0.0 {
                errors.push(format!(
                    "{label}.catalog.{item_id}: dimensions must be positive and finite"
                ));
                continue;
            }
            let size = ItemSize { width, depth };
            sizes.insert(item_id.to_owned(), size);

            let Some(envelopes) = array(item, "functional_envelopes") else {
                errors.push(format!(
                    "{label}.catalog.{item_id}: missing functional_envelopes array"
                ));
                continue;
            };
            for envelope in envelopes {
                let Some(envelope_id) = text(envelope, "envelope_id") else {
                    errors.push(format!("{label}.catalog.{item_id}: missing envelope_id"));
                    continue;
                };
                if !envelope_ids.insert(envelope_id.to_owned()) {
                    errors.push(format!("{label}: duplicate envelope_id {envelope_id:?}"));
                }
                envelope_definitions
                    .entry(envelope_id.to_owned())
                    .or_insert_with(|| EnvelopeDefinition {
                        envelope,
                        owner_id: item_id.to_owned(),
                        owner_size: size,
                    });
                if !matches!(
                    text(envelope, "task_kind"),
                    Some("drawer_open" | "chair_pull_back" | "service_access")
                ) {
                    errors.push(format!("{label}.{envelope_id}: unsupported task_kind"));
                }
                if !matches!(
                    text(envelope, "evidence_state"),
                    Some("synthetic_known_truth" | "unknown")
                ) {
                    errors.push(format!("{label}.{envelope_id}: unsupported evidence_state"));
                }
                let Some(local) =
                    field(envelope, "rect_local_item_frame").and_then(Rect::from_value)
                else {
                    errors.push(format!(
                        "{label}.{envelope_id}: malformed local envelope rectangle"
                    ));
                    continue;
                };
                if !local.is_valid()
                    || (local.y - depth).abs() > 1e-9
                    || local.x + local.width > width + 1e-9
                {
                    errors.push(format!(
                        "{label}.{envelope_id}: front envelope must start at the item's local front edge and remain within its width"
                    ));
                }
            }
        }

        let Some(requirements) = array(scenario, "requirements") else {
            errors.push(format!("{label}: missing requirements"));
            continue;
        };
        let requirement_ids = unique_ids(
            requirements,
            "requirement_id",
            &format!("{label}.requirements"),
            &mut errors,
        );
        let requirements_by_id: BTreeMap<String, &Value> = requirements
            .iter()
            .filter_map(|requirement| {
                text(requirement, "requirement_id")
                    .map(|requirement_id| (requirement_id.to_owned(), requirement))
            })
            .collect();
        let mut mandatory_unknown = false;
        for requirement in requirements {
            let Some(requirement_id) = text(requirement, "requirement_id") else {
                continue;
            };
            if text(requirement, "kind") != Some("functional_envelope_clear") {
                errors.push(format!(
                    "{label}.{requirement_id}: unsupported requirement kind"
                ));
            }
            let Some(envelope_id) = text(requirement, "target_envelope_id") else {
                errors.push(format!(
                    "{label}.{requirement_id}: missing target_envelope_id"
                ));
                continue;
            };
            let Some(definition) = envelope_definitions.get(envelope_id) else {
                errors.push(format!(
                    "{label}.{requirement_id}: unknown target envelope {envelope_id:?}"
                ));
                continue;
            };
            let envelope = definition.envelope;
            if field(requirement, "must_be_satisfied_to_advance")
                .and_then(Value::as_bool)
                .is_none()
            {
                errors.push(format!(
                    "{label}.{requirement_id}: must_be_satisfied_to_advance must be boolean"
                ));
            }
            if field(requirement, "must_be_satisfied_to_advance").and_then(Value::as_bool)
                == Some(true)
                && text(envelope, "evidence_state") == Some("unknown")
            {
                mandatory_unknown = true;
            }
        }

        let Some(alternatives) = array(scenario, "alternatives") else {
            errors.push(format!("{label}: missing alternatives"));
            continue;
        };
        if alternatives.is_empty() {
            errors.push(format!("{label}: alternatives must not be empty"));
        }
        unique_ids(
            alternatives,
            "alternative_id",
            &format!("{label}.alternatives"),
            &mut errors,
        );

        for alternative in alternatives {
            let Some(alternative_id) = text(alternative, "alternative_id") else {
                continue;
            };
            let Some(placements) = array(alternative, "placements") else {
                errors.push(format!("{label}.{alternative_id}: missing placements"));
                continue;
            };
            let placement_ids = unique_ids(
                placements,
                "item_id",
                &format!("{label}.{alternative_id}.placements"),
                &mut errors,
            );
            let placements_by_id: BTreeMap<String, &Value> = placements
                .iter()
                .filter_map(|placement| {
                    text(placement, "item_id").map(|item_id| (item_id.to_owned(), placement))
                })
                .collect();
            let mut footprints = Vec::new();
            for placement in placements {
                let Some(item_id) = text(placement, "item_id") else {
                    continue;
                };
                if !catalog_ids.contains(item_id) {
                    errors.push(format!(
                        "{label}.{alternative_id}: unknown item {item_id:?}"
                    ));
                    continue;
                }
                let Some(rotation) = field(placement, "rotation_degrees").and_then(Value::as_u64)
                else {
                    errors.push(format!(
                        "{label}.{alternative_id}.{item_id}: missing numeric rotation_degrees"
                    ));
                    continue;
                };
                if ![0, 90, 180, 270].contains(&rotation) {
                    errors.push(format!(
                        "{label}.{alternative_id}.{item_id}: unsupported rotation {rotation}"
                    ));
                    continue;
                }
                let Some((placed_id, rect)) = placement_rect(placement, &sizes) else {
                    errors.push(format!(
                        "{label}.{alternative_id}.{item_id}: cannot derive footprint"
                    ));
                    continue;
                };
                if !rect.is_valid() || !rect.inside_room(room_width, room_depth) {
                    errors.push(format!(
                        "{label}.{alternative_id}.{item_id}: footprint is invalid or outside room"
                    ));
                }
                footprints.push((placed_id, rect));
            }
            let footprint_index = SpatialIndex::new(&footprints);
            for (left, right) in footprint_index.overlapping_pairs() {
                errors.push(format!(
                    "{label}.{alternative_id}: physical footprints {} and {} overlap",
                    footprints[left].0, footprints[right].0
                ));
            }

            let Some(checks) = array(alternative, "expected_checks") else {
                errors.push(format!("{label}.{alternative_id}: missing expected_checks"));
                continue;
            };
            let check_ids = unique_ids(
                checks,
                "requirement_id",
                &format!("{label}.{alternative_id}.expected_checks"),
                &mut errors,
            );
            for check in checks {
                let Some(requirement_id) = text(check, "requirement_id") else {
                    continue;
                };
                if !requirement_ids.contains(requirement_id) {
                    errors.push(format!("{label}.{alternative_id}: check references unknown requirement {requirement_id:?}"));
                    continue;
                }
                let Some(requirement) = requirements_by_id.get(requirement_id) else {
                    continue;
                };
                let actual = expected_functional_envelope_outcome(
                    scenario,
                    requirement,
                    &envelope_definitions,
                    &placements_by_id,
                    &footprint_index,
                );
                if let Some(actual) = actual
                    && text(check, "outcome") != Some(actual)
                {
                    errors.push(format!(
                        "{label}.{alternative_id}.{requirement_id}: expected outcome should be {actual:?}, got {:?}",
                        text(check, "outcome")
                    ));
                } else if actual.is_none() {
                    errors.push(format!(
                        "{label}.{alternative_id}.{requirement_id}: outcome is indeterminate because the envelope or placement is malformed"
                    ));
                }
            }

            let mandatory_failure = checks.iter().any(|check| {
                let Some(requirement_id) = text(check, "requirement_id") else {
                    return false;
                };
                requirements_by_id
                    .get(requirement_id)
                    .is_some_and(|requirement| {
                        field(requirement, "must_be_satisfied_to_advance").and_then(Value::as_bool)
                            == Some(true)
                            && matches!(text(check, "outcome"), Some("fail" | "indeterminate"))
                    })
            });
            if mandatory_failure && text(alternative, "promotion_disposition") != Some("blocked") {
                errors.push(format!(
                    "{label}.{alternative_id}: mandatory envelope conflict/unknown must remain blocked"
                ));
            }
            if !matches!(
                text(alternative, "promotion_disposition"),
                Some("blocked" | "candidate_only")
            ) {
                errors.push(format!(
                    "{label}.{alternative_id}: invalid promotion_disposition"
                ));
            }
            if placement_ids.len() != placements.len() {
                errors.push(format!(
                    "{label}.{alternative_id}: duplicate item placement"
                ));
            }
            if check_ids.len() != checks.len() {
                errors.push(format!(
                    "{label}.{alternative_id}: duplicate requirement check"
                ));
            }
        }

        if mandatory_unknown
            && alternatives
                .iter()
                .any(|alternative| text(alternative, "promotion_disposition") != Some("blocked"))
        {
            errors.push(format!("{label}: unknown mandatory functional-envelope evidence must block every alternative"));
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::{Rect, SpatialIndex, audit_document, audit_fixtures, audit_functional_envelopes};
    use serde_json::{Value, json};
    use std::{fs, path::PathBuf};

    fn baseline() -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/interiors/furnishing_layout_scenarios_v0_1.json");
        let bytes = fs::read(&path).expect("synthetic furnishing fixtures must exist");
        serde_json::from_slice(&bytes).expect("synthetic furnishing fixtures must be JSON")
    }

    fn errors_after(mut document: Value, mutate: impl FnOnce(&mut Value)) -> Vec<String> {
        mutate(&mut document);
        audit_fixtures(&document)
    }

    fn functional_errors_after(
        mut document: Value,
        mutate: impl FnOnce(&mut Value),
    ) -> Vec<String> {
        mutate(&mut document);
        audit_functional_envelopes(&document)
    }

    #[test]
    fn frozen_known_truth_fixtures_pass() {
        let errors = audit_fixtures(&baseline());
        assert!(errors.is_empty(), "fixture audit errors: {errors:#?}");
    }

    #[test]
    fn duplicate_scenario_identity_is_rejected() {
        let errors = errors_after(baseline(), |doc| {
            let first = doc["scenarios"][0].clone();
            doc["scenarios"].as_array_mut().unwrap().push(first);
        });
        assert!(
            errors
                .iter()
                .any(|error| error.contains("duplicate scenario_id"))
        );
    }

    #[test]
    fn unknown_catalog_item_is_rejected() {
        let errors = errors_after(baseline(), |doc| {
            doc["scenarios"][0]["alternatives"][0]["placements"][0]["item_id"] =
                json!("phantom-chair");
        });
        assert!(errors.iter().any(|error| error.contains("unknown item")));
    }

    #[test]
    fn rotated_out_of_bounds_footprint_is_rejected() {
        let errors = errors_after(baseline(), |doc| {
            doc["scenarios"][3]["alternatives"][0]["placements"][1]["x_m"] = json!(3.0);
        });
        assert!(
            errors
                .iter()
                .any(|error| error.contains("outside nominal room"))
        );
    }

    #[test]
    fn expected_clear_route_outcome_cannot_be_forged() {
        let errors = errors_after(baseline(), |doc| {
            doc["scenarios"][0]["alternatives"][0]["expected_checks"][0]["outcome"] = json!("pass");
        });
        assert!(
            errors
                .iter()
                .any(|error| error.contains("expected outcome should be \"fail\""))
        );
    }

    #[test]
    fn unknown_site_geometry_cannot_be_claimed_as_pass() {
        let errors = errors_after(baseline(), |doc| {
            doc["scenarios"][3]["alternatives"][0]["expected_checks"][0]["outcome"] = json!("pass");
        });
        assert!(
            errors
                .iter()
                .any(|error| error.contains("expected outcome should be \"indeterminate\""))
        );
    }

    #[test]
    fn mandatory_unknown_blocks_every_alternative() {
        let errors = errors_after(baseline(), |doc| {
            doc["scenarios"][3]["alternatives"][1]["promotion_disposition"] =
                json!("candidate_only");
        });
        assert!(
            errors
                .iter()
                .any(|error| error.contains("must block every alternative"))
        );
    }

    #[test]
    fn furniture_overlap_is_detected() {
        let errors = errors_after(baseline(), |doc| {
            doc["scenarios"][0]["alternatives"][1]["placements"][2]["x_m"] = json!(1.5);
            doc["scenarios"][0]["alternatives"][1]["placements"][2]["y_m"] = json!(3.0);
        });
        assert!(
            errors
                .iter()
                .any(|error| error.contains("furniture bed overlaps console")
                    || error.contains("furniture console overlaps bed"))
        );
    }

    #[test]
    fn hard_constraint_failure_cannot_be_promoted() {
        let errors = errors_after(baseline(), |doc| {
            doc["scenarios"][0]["alternatives"][0]["promotion_disposition"] =
                json!("candidate_only");
        });
        assert!(
            errors
                .iter()
                .any(|error| error.contains("must remain blocked"))
        );
    }

    #[test]
    fn household_tradeoff_is_incomparable_not_scalar_ranked() {
        let doc = baseline();
        let scenario = &doc["scenarios"][2];
        let left = &scenario["alternatives"][0];
        let right = &scenario["alternatives"][1];
        let relation = super::preference_relation(left, right);
        assert_eq!(relation, Some("incomparable"));
    }

    #[test]
    fn unit_and_coordinate_contract_is_not_ambiguous() {
        let errors = errors_after(baseline(), |doc| {
            doc["units"]["length"] = json!("feet");
        });
        assert!(errors.iter().any(|error| error.contains("units")));
    }
    fn functional_baseline() -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/interiors/furnishing_functional_envelopes_v0_2.json");
        let bytes = fs::read(&path).expect("functional-envelope fixtures must exist");
        serde_json::from_slice(&bytes).expect("functional-envelope fixtures must be JSON")
    }

    #[test]
    fn spatial_index_matches_brute_force_for_overlaps_and_queries() {
        let mut items = Vec::new();
        for row in 0..17 {
            for column in 0..19 {
                let width = 0.15 + f64::from((row + column) % 4) * 0.09;
                let depth = 0.12 + f64::from((row * 3 + column) % 5) * 0.07;
                let rect = Rect {
                    x: f64::from(column) * 0.16,
                    y: f64::from(row) * 0.18,
                    width,
                    depth,
                };
                items.push((format!("item-{row}-{column}"), rect));
            }
        }
        let index = SpatialIndex::new(&items);
        let actual_pairs = index.overlapping_pairs();
        let mut expected_pairs = Vec::new();
        for left in 0..items.len() {
            for right in (left + 1)..items.len() {
                if items[left].1.overlaps(items[right].1) {
                    expected_pairs.push((left, right));
                }
            }
        }
        assert_eq!(actual_pairs, expected_pairs);

        for query_row in 0..13 {
            for query_column in 0..15 {
                let query = Rect {
                    x: f64::from(query_column) * 0.19 + 0.04,
                    y: f64::from(query_row) * 0.21 + 0.03,
                    width: 0.37,
                    depth: 0.29,
                };
                let mut actual = index.query_indices(query);
                actual.sort_unstable();
                let expected = items
                    .iter()
                    .enumerate()
                    .filter(|(_, (_, rect))| rect.overlaps(query))
                    .map(|(item_index, _)| item_index)
                    .collect::<Vec<_>>();
                assert_eq!(actual, expected);
            }
        }
    }

    #[test]
    fn frozen_functional_envelope_scenarios_pass() {
        let errors = audit_functional_envelopes(&functional_baseline());
        assert!(
            errors.is_empty(),
            "functional-envelope audit errors: {errors:#?}"
        );
    }

    #[test]
    fn drawer_access_collision_cannot_be_forged_as_clear() {
        let errors = functional_errors_after(functional_baseline(), |doc| {
            doc["scenarios"][0]["alternatives"][1]["expected_checks"][0]["outcome"] = json!("pass");
        });
        assert!(
            errors
                .iter()
                .any(|error| error.contains("expected outcome should be \"fail\""))
        );
    }

    #[test]
    fn quarter_turn_rotates_the_functional_envelope_with_its_item() {
        let errors = functional_errors_after(functional_baseline(), |doc| {
            doc["scenarios"][1]["alternatives"][1]["placements"][1]["x_m"] = json!(0.9);
        });
        assert!(
            errors
                .iter()
                .any(|error| error.contains("expected outcome should be \"pass\""))
        );
    }

    #[test]
    fn unknown_mandatory_envelope_evidence_cannot_be_promoted() {
        let errors = functional_errors_after(functional_baseline(), |doc| {
            doc["scenarios"][2]["alternatives"][0]["promotion_disposition"] =
                json!("candidate_only");
        });
        assert!(
            errors
                .iter()
                .any(|error| error.contains("unknown mandatory functional-envelope evidence"))
        );
        assert!(
            errors
                .iter()
                .any(|error| error.contains("must remain blocked"))
        );
    }

    #[test]
    fn out_of_room_functional_envelope_is_a_hard_failure() {
        let errors = audit_functional_envelopes(&functional_baseline());
        assert!(
            errors.is_empty(),
            "outside-room fixture must have a correct blocked oracle: {errors:#?}"
        );
        let errors = functional_errors_after(functional_baseline(), |doc| {
            doc["scenarios"][3]["alternatives"][0]["expected_checks"][0]["outcome"] = json!("pass");
        });
        assert!(
            errors
                .iter()
                .any(|error| error.contains("expected outcome should be \"fail\""))
        );
    }

    #[test]
    fn duplicate_functional_envelope_id_is_rejected() {
        let errors = functional_errors_after(functional_baseline(), |doc| {
            doc["scenarios"][0]["catalog"][1]["functional_envelopes"] = json!([{
                "envelope_id":"drawer_front",
                "task_kind":"drawer_open",
                "evidence_state":"synthetic_known_truth",
                "rect_local_item_frame":{"x_m":0.0,"y_m":0.45,"width_m":0.45,"depth_m":0.5}
            }]);
        });
        assert!(
            errors
                .iter()
                .any(|error| error.contains("duplicate envelope_id"))
        );
    }

    #[test]
    fn unknown_schema_version_fails_closed_without_falling_back_to_v01() {
        let mut doc = functional_baseline();
        doc["schema_version"] = json!("0.9.9");
        assert!(
            audit_document(&doc)
                .iter()
                .any(|error| error.contains("unsupported schema_version"))
        );
    }
}

//! Independent semantic oracle for Symtropy furnishing-layout fixtures.
//!
//! This crate audits known-truth synthetic geometry; it does not establish
//! building-code compliance, physical safety, measured-site truth, or usability.

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

#[derive(Clone, Copy, Debug)]
struct ItemSize {
    width: f64,
    depth: f64,
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

#[cfg(test)]
mod tests {
    use super::audit_fixtures;
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
}

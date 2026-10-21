//! Deterministic single-item repair suggestions for known-truth furnishing fixtures.
//!
//! This is a bounded proposal generator, not a verifier. It suggests one-item
//! translations around room/object/zone boundaries, preserves orientation, and
//! requires the ordinary independent fixture auditor to re-check the exact edit.
//! It abstains when mandatory evidence or supported hard-constraint semantics are unknown.

use serde_json::Value;

const OVERLAP_EPSILON_M: f64 = 1e-9;
const SEPARATION_MARGIN_M: f64 = 4e-9;
const MAX_CANDIDATE_TESTS: usize = 100_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepairDisposition {
    SuggestionsAvailable,
    NoBlockingItem,
    NoRepairFound,
    Indeterminate,
    Blocked,
    SearchCapped,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RepairSuggestion {
    pub item_id: String,
    pub from_x_m: f64,
    pub from_y_m: f64,
    pub to_x_m: f64,
    pub to_y_m: f64,
    pub displacement_m: f64,
    pub changed_fields: [&'static str; 2],
    pub verification_requirement: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RepairReport {
    pub disposition: RepairDisposition,
    pub scenario_id: String,
    pub alternative_id: String,
    pub requirement_id: String,
    pub blocking_item_ids: Vec<String>,
    pub candidate_count_before_limit: usize,
    pub suggestions: Vec<RepairSuggestion>,
    pub diagnostics: Vec<String>,
    pub claim_ceiling: &'static str,
}

#[derive(Clone, Copy, Debug)]
struct Rect {
    x: f64,
    y: f64,
    width: f64,
    depth: f64,
}

impl Rect {
    fn valid(self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.width.is_finite()
            && self.depth.is_finite()
            && self.width > 0.0
            && self.depth > 0.0
    }

    fn inside_room(self, width: f64, depth: f64) -> bool {
        self.x >= 0.0
            && self.y >= 0.0
            && self.x + self.width <= width + OVERLAP_EPSILON_M
            && self.y + self.depth <= depth + OVERLAP_EPSILON_M
    }

    fn overlaps(self, other: Self) -> bool {
        self.x < other.x + other.width - OVERLAP_EPSILON_M
            && self.x + self.width > other.x + OVERLAP_EPSILON_M
            && self.y < other.y + other.depth - OVERLAP_EPSILON_M
            && self.y + self.depth > other.y + OVERLAP_EPSILON_M
    }
}

#[derive(Clone, Debug)]
struct ItemPlacement {
    item_id: String,
    x: f64,
    y: f64,
    width: f64,
    depth: f64,
}

impl ItemPlacement {
    fn rect(&self) -> Rect {
        Rect {
            x: self.x,
            y: self.y,
            width: self.width,
            depth: self.depth,
        }
    }
}

fn string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key)?.as_str()
}

fn number(value: &Value, key: &str) -> Option<f64> {
    let result = value.get(key)?.as_f64()?;
    result.is_finite().then_some(result)
}

fn boolean(value: &Value, key: &str) -> Option<bool> {
    value.get(key)?.as_bool()
}

fn rect_from_value(value: &Value) -> Option<Rect> {
    Some(Rect {
        x: number(value, "x_m")?,
        y: number(value, "y_m")?,
        width: number(value, "width_m")?,
        depth: number(value, "depth_m")?,
    })
}

#[derive(Clone, Copy)]
struct RepairTarget<'a> {
    scenario_id: &'a str,
    alternative_id: &'a str,
    requirement_id: &'a str,
}

fn outcome_report(
    disposition: RepairDisposition,
    target: RepairTarget<'_>,
    blocking_item_ids: Vec<String>,
    candidate_count_before_limit: usize,
    suggestions: Vec<RepairSuggestion>,
    diagnostics: Vec<String>,
) -> RepairReport {
    RepairReport {
        disposition,
        scenario_id: target.scenario_id.to_owned(),
        alternative_id: target.alternative_id.to_owned(),
        requirement_id: target.requirement_id.to_owned(),
        blocking_item_ids,
        candidate_count_before_limit,
        suggestions,
        diagnostics,
        claim_ceiling: "Advisory single-item translations on synthetic known-truth 2D rectangles. Every returned edit must be re-run through the normal independent audit before any layout is considered feasible.",
    }
}

fn push_coordinate(values: &mut Vec<f64>, value: f64) {
    if value.is_finite() && !values.contains(&value) {
        values.push(value);
    }
}

fn item_rect(placement: &Value, catalog: &[Value]) -> Option<ItemPlacement> {
    let item_id = string(placement, "item_id")?;
    let item = catalog
        .iter()
        .find(|item| string(item, "item_id") == Some(item_id))?;
    let item_width = number(item, "width_m")?;
    let item_depth = number(item, "depth_m")?;
    let x = number(placement, "x_m")?;
    let y = number(placement, "y_m")?;
    let rotation = placement.get("rotation_degrees")?.as_u64()?;
    let (width, depth) = match rotation {
        0 | 180 => (item_width, item_depth),
        90 | 270 => (item_depth, item_width),
        _ => return None,
    };
    let result = ItemPlacement {
        item_id: item_id.to_owned(),
        x,
        y,
        width,
        depth,
    };
    result.rect().valid().then_some(result)
}

fn target_zone<'a>(scenario: &'a Value, requirement: &Value) -> Option<&'a Value> {
    let zone_id = string(requirement, "target_zone_id")?;
    scenario
        .get("zones")?
        .as_array()?
        .iter()
        .find(|zone| string(zone, "zone_id") == Some(zone_id))
}

fn hard_evidence_is_known(scenario: &Value) -> bool {
    if string(scenario, "scope") != Some("synthetic_only") {
        return false;
    }
    if string(
        scenario.get("room").unwrap_or(&Value::Null),
        "geometry_evidence_state",
    ) != Some("synthetic_known_truth")
    {
        return false;
    }
    let Some(requirements) = scenario.get("requirements").and_then(Value::as_array) else {
        return false;
    };
    let Some(zones) = scenario.get("zones").and_then(Value::as_array) else {
        return false;
    };

    for requirement in requirements {
        if boolean(requirement, "must_be_satisfied_to_advance") != Some(true) {
            continue;
        }
        if string(requirement, "classification") != Some("hard_constraint")
            || string(requirement, "evidence_state") != Some("synthetic_known_truth")
        {
            return false;
        }
        match string(requirement, "kind") {
            Some("zone_must_remain_clear") | Some("item_must_avoid_zone") => {
                let Some(zone_id) = string(requirement, "target_zone_id") else {
                    return false;
                };
                if !zones.iter().any(|zone| {
                    string(zone, "zone_id") == Some(zone_id)
                        && string(zone, "evidence_state") == Some("synthetic_known_truth")
                        && zone
                            .get("rect")
                            .and_then(rect_from_value)
                            .is_some_and(Rect::valid)
                }) {
                    return false;
                }
            }
            Some("required_geometry_evidence") => {}
            _ => return false,
        }
    }
    true
}

fn mandatory_constraints_pass(scenario: &Value, placements: &[ItemPlacement]) -> bool {
    let Some(requirements) = scenario.get("requirements").and_then(Value::as_array) else {
        return false;
    };
    let room = scenario.get("room").unwrap_or(&Value::Null);
    let room_width = number(room, "width_m");
    let room_depth = number(room, "depth_m");

    for requirement in requirements {
        if boolean(requirement, "must_be_satisfied_to_advance") != Some(true) {
            continue;
        }
        if string(requirement, "evidence_state") != Some("synthetic_known_truth")
            || string(requirement, "classification") != Some("hard_constraint")
        {
            return false;
        }
        match string(requirement, "kind") {
            Some("required_geometry_evidence") => {
                if string(room, "geometry_evidence_state") != Some("synthetic_known_truth") {
                    return false;
                }
            }
            Some("zone_must_remain_clear") => {
                let Some(zone) = target_zone(scenario, requirement) else {
                    return false;
                };
                let Some(zone_rect) = zone.get("rect").and_then(rect_from_value) else {
                    return false;
                };
                if placements
                    .iter()
                    .any(|placement| placement.rect().overlaps(zone_rect))
                {
                    return false;
                }
            }
            Some("item_must_avoid_zone") => {
                let Some(zone) = target_zone(scenario, requirement) else {
                    return false;
                };
                let Some(zone_rect) = zone.get("rect").and_then(rect_from_value) else {
                    return false;
                };
                let Some(item_id) = string(requirement, "target_item_id") else {
                    return false;
                };
                let Some(placement) = placements.iter().find(|item| item.item_id == item_id) else {
                    return false;
                };
                if placement.rect().overlaps(zone_rect) {
                    return false;
                }
            }
            _ => return false,
        }
    }
    let (Some(room_width), Some(room_depth)) = (room_width, room_depth) else {
        return false;
    };
    placements.iter().all(|placement| {
        placement.rect().valid() && placement.rect().inside_room(room_width, room_depth)
    })
}

fn candidate_positions(
    moved: &ItemPlacement,
    room_width: f64,
    room_depth: f64,
    zone_rects: &[Rect],
    other_rects: &[Rect],
) -> Option<Vec<(f64, f64)>> {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    push_coordinate(&mut xs, 0.0);
    push_coordinate(&mut ys, 0.0);
    push_coordinate(&mut xs, moved.x);
    push_coordinate(&mut ys, moved.y);
    push_coordinate(&mut xs, room_width - moved.width);
    push_coordinate(&mut ys, room_depth - moved.depth);

    for rect in zone_rects.iter().chain(other_rects.iter()) {
        push_coordinate(&mut xs, rect.x - moved.width - SEPARATION_MARGIN_M);
        push_coordinate(&mut xs, rect.x + rect.width + SEPARATION_MARGIN_M);
        push_coordinate(&mut ys, rect.y - moved.depth - SEPARATION_MARGIN_M);
        push_coordinate(&mut ys, rect.y + rect.depth + SEPARATION_MARGIN_M);
    }

    if xs.len().saturating_mul(ys.len()) > MAX_CANDIDATE_TESTS {
        return None;
    }

    let mut candidates = Vec::with_capacity(xs.len() * ys.len());
    for x in xs {
        for y in &ys {
            candidates.push((x, *y));
        }
    }
    Some(candidates)
}

/// Propose deterministic one-item translations to satisfy one known hard zone-clearance
/// requirement. This function intentionally does not certify the result: callers must
/// apply a proposed edit to the exact design revision and run the normal oracle again.
///
/// The current v0.1 search preserves item identity, footprint dimensions, orientation,
/// other placements, and all mandatory hard constraints supported by this geometry model.
pub fn suggest_single_item_zone_repairs(
    document: &Value,
    scenario_id: &str,
    alternative_id: &str,
    requirement_id: &str,
    limit: usize,
) -> RepairReport {
    let target = RepairTarget {
        scenario_id,
        alternative_id,
        requirement_id,
    };
    if limit == 0 {
        return outcome_report(
            RepairDisposition::Blocked,
            target,
            Vec::new(),
            0,
            Vec::new(),
            vec!["limit must be greater than zero".to_owned()],
        );
    }

    let structural_errors = super::audit_document(document);
    if !structural_errors.is_empty() {
        return outcome_report(
            RepairDisposition::Blocked,
            target,
            Vec::new(),
            0,
            Vec::new(),
            structural_errors,
        );
    }
    if string(document, "schema_version") != Some("0.1.0") {
        return outcome_report(
            RepairDisposition::Blocked,
            target,
            Vec::new(),
            0,
            Vec::new(),
            vec![
                "repair v0.1 supports only the known-truth v0.1 axis-aligned fixture contract"
                    .to_owned(),
            ],
        );
    }

    let scenarios = document.get("scenarios").and_then(Value::as_array);
    let Some(scenario) = scenarios.and_then(|items| {
        items
            .iter()
            .find(|item| string(item, "scenario_id") == Some(scenario_id))
    }) else {
        return outcome_report(
            RepairDisposition::Blocked,
            target,
            Vec::new(),
            0,
            Vec::new(),
            vec!["scenario_id does not exist in the audited document".to_owned()],
        );
    };

    if !hard_evidence_is_known(scenario) {
        return outcome_report(
            RepairDisposition::Indeterminate,
            target,
            Vec::new(),
            0,
            Vec::new(),
            vec!["one or more mandatory hard constraints lack supported known-truth evidence; no repair is promoted".to_owned()],
        );
    }

    let Some(requirements) = scenario.get("requirements").and_then(Value::as_array) else {
        return outcome_report(
            RepairDisposition::Blocked,
            target,
            Vec::new(),
            0,
            Vec::new(),
            vec!["scenario requirements are missing".to_owned()],
        );
    };
    let Some(requirement) = requirements
        .iter()
        .find(|item| string(item, "requirement_id") == Some(requirement_id))
    else {
        return outcome_report(
            RepairDisposition::Blocked,
            target,
            Vec::new(),
            0,
            Vec::new(),
            vec!["requirement_id does not exist in the selected scenario".to_owned()],
        );
    };

    if string(requirement, "kind") != Some("zone_must_remain_clear")
        || string(requirement, "classification") != Some("hard_constraint")
        || boolean(requirement, "must_be_satisfied_to_advance") != Some(true)
        || string(requirement, "evidence_state") != Some("synthetic_known_truth")
    {
        return outcome_report(
            RepairDisposition::Indeterminate,
            target,
            Vec::new(),
            0,
            Vec::new(),
            vec![
                "target must be a mandatory, known-truth zone_must_remain_clear hard constraint"
                    .to_owned(),
            ],
        );
    }

    let Some(zone) = target_zone(scenario, requirement) else {
        return outcome_report(
            RepairDisposition::Blocked,
            target,
            Vec::new(),
            0,
            Vec::new(),
            vec!["target zone is missing".to_owned()],
        );
    };
    if string(zone, "evidence_state") != Some("synthetic_known_truth") {
        return outcome_report(
            RepairDisposition::Indeterminate,
            target,
            Vec::new(),
            0,
            Vec::new(),
            vec!["target zone evidence is not known-truth".to_owned()],
        );
    }
    let Some(zone_rect) = zone.get("rect").and_then(rect_from_value) else {
        return outcome_report(
            RepairDisposition::Blocked,
            target,
            Vec::new(),
            0,
            Vec::new(),
            vec!["target zone rectangle is malformed".to_owned()],
        );
    };

    let Some(alternatives) = scenario.get("alternatives").and_then(Value::as_array) else {
        return outcome_report(
            RepairDisposition::Blocked,
            target,
            Vec::new(),
            0,
            Vec::new(),
            vec!["scenario alternatives are missing".to_owned()],
        );
    };
    let Some(alternative) = alternatives
        .iter()
        .find(|item| string(item, "alternative_id") == Some(alternative_id))
    else {
        return outcome_report(
            RepairDisposition::Blocked,
            target,
            Vec::new(),
            0,
            Vec::new(),
            vec!["alternative_id does not exist in the selected scenario".to_owned()],
        );
    };

    let Some(room) = scenario.get("room") else {
        return outcome_report(
            RepairDisposition::Blocked,
            target,
            Vec::new(),
            0,
            Vec::new(),
            vec!["room geometry is missing".to_owned()],
        );
    };
    let (Some(room_width), Some(room_depth)) = (number(room, "width_m"), number(room, "depth_m"))
    else {
        return outcome_report(
            RepairDisposition::Blocked,
            target,
            Vec::new(),
            0,
            Vec::new(),
            vec!["room dimensions are malformed".to_owned()],
        );
    };
    let Some(catalog) = scenario.get("catalog").and_then(Value::as_array) else {
        return outcome_report(
            RepairDisposition::Blocked,
            target,
            Vec::new(),
            0,
            Vec::new(),
            vec!["catalog is missing".to_owned()],
        );
    };
    let Some(placements_json) = alternative.get("placements").and_then(Value::as_array) else {
        return outcome_report(
            RepairDisposition::Blocked,
            target,
            Vec::new(),
            0,
            Vec::new(),
            vec!["alternative placements are missing".to_owned()],
        );
    };

    let placements = placements_json
        .iter()
        .filter_map(|placement| item_rect(placement, catalog))
        .collect::<Vec<_>>();
    if placements.len() != placements_json.len() {
        return outcome_report(
            RepairDisposition::Blocked,
            target,
            Vec::new(),
            0,
            Vec::new(),
            vec!["one or more placements cannot be parsed into finite rectangles".to_owned()],
        );
    }
    let blockers = placements
        .iter()
        .filter(|placement| placement.rect().overlaps(zone_rect))
        .map(|placement| placement.item_id.clone())
        .collect::<Vec<_>>();
    if blockers.is_empty() {
        return outcome_report(
            RepairDisposition::NoBlockingItem,
            target,
            blockers,
            0,
            Vec::new(),
            vec!["the target zone is already clear; no relocation is proposed".to_owned()],
        );
    }

    let mut protected_zones = Vec::new();
    if let Some(zones) = scenario.get("zones").and_then(Value::as_array) {
        for candidate_zone in zones {
            if matches!(
                string(candidate_zone, "kind"),
                Some("keep_clear" | "service_access" | "no_place")
            ) && let Some(rect) = candidate_zone.get("rect").and_then(rect_from_value)
            {
                protected_zones.push(rect);
            }
        }
    }

    let mut candidates = Vec::new();
    let mut candidate_tests = 0usize;
    for blocking_id in &blockers {
        let Some(moved) = placements.iter().find(|item| &item.item_id == blocking_id) else {
            continue;
        };
        let other_rects = placements
            .iter()
            .filter(|item| item.item_id != *blocking_id)
            .map(ItemPlacement::rect)
            .collect::<Vec<_>>();
        let Some(positions) = candidate_positions(
            moved,
            room_width,
            room_depth,
            &protected_zones,
            &other_rects,
        ) else {
            return outcome_report(
                RepairDisposition::SearchCapped,
            target,
                blockers,
                0,
                Vec::new(),
                vec![format!(
                    "candidate search would exceed {MAX_CANDIDATE_TESTS} position checks"
                )],
            );
        };

        if candidate_tests.saturating_add(positions.len()) > MAX_CANDIDATE_TESTS {
            return outcome_report(
                RepairDisposition::SearchCapped,
            target,
                blockers,
                0,
                Vec::new(),
                vec![format!(
                    "total candidate search would exceed {MAX_CANDIDATE_TESTS} position checks"
                )],
            );
        }
        candidate_tests = candidate_tests.saturating_add(positions.len());
        for (x, y) in positions {
            if (x - moved.x).abs() <= f64::EPSILON && (y - moved.y).abs() <= f64::EPSILON {
                continue;
            }
            let candidate_rect = Rect {
                x,
                y,
                width: moved.width,
                depth: moved.depth,
            };
            if !candidate_rect.valid() || !candidate_rect.inside_room(room_width, room_depth) {
                continue;
            }
            if candidate_rect.overlaps(zone_rect)
                || protected_zones
                    .iter()
                    .any(|zone| candidate_rect.overlaps(*zone))
                || other_rects
                    .iter()
                    .any(|other| candidate_rect.overlaps(*other))
            {
                continue;
            }

            let mut updated = placements.clone();
            if let Some(placement) = updated
                .iter_mut()
                .find(|item| item.item_id == moved.item_id)
            {
                placement.x = x;
                placement.y = y;
            }
            if !mandatory_constraints_pass(scenario, &updated) {
                continue;
            }

            let displacement_m = (x - moved.x).hypot(y - moved.y);
            candidates.push(RepairSuggestion {
                item_id: moved.item_id.clone(),
                from_x_m: moved.x,
                from_y_m: moved.y,
                to_x_m: x,
                to_y_m: y,
                displacement_m,
                changed_fields: ["x_m", "y_m"],
                verification_requirement: "rerun the normal audit on the exact edited design revision; this suggestion itself is not a certificate",
            });
        }
    }

    candidates.sort_by(|left, right| {
        left.displacement_m
            .total_cmp(&right.displacement_m)
            .then_with(|| left.item_id.cmp(&right.item_id))
            .then_with(|| left.to_x_m.total_cmp(&right.to_x_m))
            .then_with(|| left.to_y_m.total_cmp(&right.to_y_m))
    });
    candidates.dedup_by(|left, right| {
        left.item_id == right.item_id && left.to_x_m == right.to_x_m && left.to_y_m == right.to_y_m
    });
    let candidate_count_before_limit = candidates.len();
    candidates.truncate(limit);

    if candidates.is_empty() {
        outcome_report(
            RepairDisposition::NoRepairFound,
            target,
            blockers,
            candidate_count_before_limit,
            candidates,
            vec![format!(
                "no single-item translation resolved the target and all supported mandatory constraints within {candidate_tests} deterministic candidate checks"
            )],
        )
    } else {
        outcome_report(
            RepairDisposition::SuggestionsAvailable,
            target,
            blockers,
            candidate_count_before_limit,
            candidates,
            vec![format!(
                "generated and locally filtered candidate moves after {candidate_tests} deterministic position checks; the independent audit must revalidate every exact edited candidate"
            )],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{RepairDisposition, suggest_single_item_zone_repairs};
    use serde_json::{Value, json};
    use std::{fs, path::PathBuf};

    fn baseline() -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/interiors/furnishing_layout_scenarios_v0_1.json");
        let bytes = fs::read(&path).expect("furnishing fixture must exist");
        serde_json::from_slice(&bytes).expect("furnishing fixture must be valid JSON")
    }

    #[test]
    fn proposes_deterministic_single_item_relocation_for_blocked_route() {
        let document = baseline();
        let report = suggest_single_item_zone_repairs(
            &document,
            "FURN-001",
            "console_blocks_route",
            "route_remains_clear",
            10,
        );
        assert_eq!(report.disposition, RepairDisposition::SuggestionsAvailable);
        assert_eq!(report.blocking_item_ids, vec!["console"]);
        assert!(!report.suggestions.is_empty());
        let first = &report.suggestions[0];
        assert_eq!(first.item_id, "console");
        assert!(first.displacement_m > 0.0);
        assert!(first.to_x_m >= 0.9);
        assert!(first.verification_requirement.contains("normal audit"));

        let replay = suggest_single_item_zone_repairs(
            &document,
            "FURN-001",
            "console_blocks_route",
            "route_remains_clear",
            10,
        );
        assert_eq!(report, replay);
    }

    #[test]
    fn returned_candidate_passes_the_existing_independent_fixture_oracle() {
        let mut document = baseline();
        let report = suggest_single_item_zone_repairs(
            &document,
            "FURN-001",
            "console_blocks_route",
            "route_remains_clear",
            1,
        );
        let suggestion = report
            .suggestions
            .first()
            .expect("a repair must be suggested");
        let scenario = &mut document["scenarios"][0];
        let alternative = &mut scenario["alternatives"][0];
        let placement = alternative["placements"]
            .as_array_mut()
            .expect("placements")
            .iter_mut()
            .find(|placement| placement["item_id"] == json!("console"))
            .expect("console placement");
        placement["x_m"] = json!(suggestion.to_x_m);
        placement["y_m"] = json!(suggestion.to_y_m);
        alternative["expected_checks"][0]["outcome"] = json!("pass");
        alternative["promotion_disposition"] = json!("candidate_only");
        assert!(
            super::super::audit_document(&document).is_empty(),
            "normal fixture oracle rejected the exact suggested edit"
        );
    }

    #[test]
    fn does_not_generate_a_repair_for_already_clear_target() {
        let document = baseline();
        let report = suggest_single_item_zone_repairs(
            &document,
            "FURN-001",
            "console_relocated",
            "route_remains_clear",
            5,
        );
        assert_eq!(report.disposition, RepairDisposition::NoBlockingItem);
        assert!(report.suggestions.is_empty());
    }

    #[test]
    fn abstains_for_unknown_real_site_geometry() {
        let document = baseline();
        let report = suggest_single_item_zone_repairs(
            &document,
            "FURN-004",
            "unresolved_retrofit",
            "required_real_geometry",
            5,
        );
        assert_eq!(report.disposition, RepairDisposition::Indeterminate);
        assert!(report.suggestions.is_empty());
    }

    #[test]
    fn invalid_input_and_zero_limit_are_blocked() {
        let document = baseline();
        let zero_limit = suggest_single_item_zone_repairs(
            &document,
            "FURN-001",
            "console_blocks_route",
            "route_remains_clear",
            0,
        );
        assert_eq!(zero_limit.disposition, RepairDisposition::Blocked);
        let mut malformed = document;
        malformed["units"]["length"] = json!("feet");
        let report = suggest_single_item_zone_repairs(
            &malformed,
            "FURN-001",
            "console_blocks_route",
            "route_remains_clear",
            5,
        );
        assert_eq!(report.disposition, RepairDisposition::Blocked);
    }
}

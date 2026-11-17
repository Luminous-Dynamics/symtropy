//! Analytic static-support reference evaluator for a declared support plane.
//!
//! This crate evaluates only the gravity projection against the convex hull of
//! declared active contacts. It is not a dynamic stability, friction, force
//! allocation, actuator, gait, viability, or motion-authority model.

#![forbid(unsafe_code)]

use std::cmp::Ordering;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupportFrame {
    pub frame_id: String,
    pub plane_id: String,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContactPoint {
    pub foot_id: String,
    pub frame_id: String,
    pub plane_id: String,
    pub frame_revision: u64,
    pub position_m: [f64; 2],
    pub uncertainty_radius_m: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SupportSnapshot {
    pub robot_id: String,
    pub morphology_revision: String,
    pub support_frame: SupportFrame,
    pub com_projection_m: [f64; 2],
    pub com_uncertainty_m: f64,
    pub required_margin_m: f64,
    pub contacts: Vec<ContactPoint>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupportDisposition {
    Stable,
    Marginal,
    Outside,
    InsufficientContacts,
    DegenerateSupport,
    InvalidEvidence,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SupportEvaluation {
    pub disposition: SupportDisposition,
    pub support_hull_m: Vec<[f64; 2]>,
    pub hull_area_m2: Option<f64>,
    pub contact_count: usize,
    pub nominal_signed_margin_m: Option<f64>,
    pub conservative_margin_m: Option<f64>,
    pub uncertainty_deduction_m: Option<f64>,
    pub required_margin_m: f64,
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Point2 {
    x: f64,
    y: f64,
}

impl Point2 {
    fn from_array(value: [f64; 2]) -> Self {
        Self {
            x: value[0],
            y: value[1],
        }
    }

    fn to_array(self) -> [f64; 2] {
        [self.x, self.y]
    }
}

fn invalid(snapshot: &SupportSnapshot, error: impl Into<String>) -> SupportEvaluation {
    SupportEvaluation {
        disposition: SupportDisposition::InvalidEvidence,
        support_hull_m: Vec::new(),
        hull_area_m2: None,
        contact_count: snapshot.contacts.len(),
        nominal_signed_margin_m: None,
        conservative_margin_m: None,
        uncertainty_deduction_m: None,
        required_margin_m: snapshot.required_margin_m,
        error: Some(error.into()),
    }
}

fn cross(origin: Point2, a: Point2, b: Point2) -> f64 {
    (a.x - origin.x) * (b.y - origin.y) - (a.y - origin.y) * (b.x - origin.x)
}

fn validate_snapshot(snapshot: &SupportSnapshot) -> Result<(), String> {
    if snapshot.robot_id.trim().is_empty() {
        return Err("robot identity is missing".to_owned());
    }
    if snapshot.morphology_revision.trim().is_empty() {
        return Err("morphology revision is missing".to_owned());
    }
    if snapshot.support_frame.frame_id.trim().is_empty()
        || snapshot.support_frame.plane_id.trim().is_empty()
    {
        return Err("support frame or plane identity is missing".to_owned());
    }
    if !snapshot
        .com_projection_m
        .iter()
        .all(|value| value.is_finite())
    {
        return Err("centre-of-mass projection is non-finite".to_owned());
    }
    if !snapshot.com_uncertainty_m.is_finite() || snapshot.com_uncertainty_m < 0.0 {
        return Err("centre-of-mass uncertainty must be finite and nonnegative".to_owned());
    }
    if !snapshot.required_margin_m.is_finite() || snapshot.required_margin_m < 0.0 {
        return Err("required support margin must be finite and nonnegative".to_owned());
    }

    let mut foot_ids = std::collections::BTreeSet::new();
    for contact in &snapshot.contacts {
        if contact.foot_id.trim().is_empty() {
            return Err("contact identity is missing".to_owned());
        }
        if !foot_ids.insert(contact.foot_id.as_str()) {
            return Err(format!("duplicate contact identity: {}", contact.foot_id));
        }
        if contact.frame_id != snapshot.support_frame.frame_id
            || contact.plane_id != snapshot.support_frame.plane_id
            || contact.frame_revision != snapshot.support_frame.revision
        {
            return Err(format!(
                "contact {} uses a stale or incompatible support frame",
                contact.foot_id
            ));
        }
        if !contact.position_m.iter().all(|value| value.is_finite()) {
            return Err(format!(
                "contact {} has a non-finite position",
                contact.foot_id
            ));
        }
        if !contact.uncertainty_radius_m.is_finite() || contact.uncertainty_radius_m < 0.0 {
            return Err(format!(
                "contact {} uncertainty must be finite and nonnegative",
                contact.foot_id
            ));
        }
    }
    Ok(())
}

fn compare_points(a: &Point2, b: &Point2) -> Ordering {
    a.x.total_cmp(&b.x).then_with(|| a.y.total_cmp(&b.y))
}

/// Deterministic monotone-chain convex hull, counter-clockwise from the
/// lexicographically smallest point. Collinear interior points are omitted.
fn convex_hull(points: &[Point2]) -> Result<Vec<Point2>, String> {
    let mut sorted = points.to_vec();
    sorted.sort_by(compare_points);
    sorted.dedup_by(|a, b| a.x == b.x && a.y == b.y);

    if sorted.len() < 3 {
        return Ok(sorted);
    }

    let mut lower = Vec::with_capacity(sorted.len());
    for point in &sorted {
        while lower.len() >= 2 {
            let turn = cross(lower[lower.len() - 2], lower[lower.len() - 1], *point);
            if !turn.is_finite() {
                return Err("convex-hull arithmetic overflowed".to_owned());
            }
            if turn > 0.0 {
                break;
            }
            lower.pop();
        }
        lower.push(*point);
    }

    let mut upper = Vec::with_capacity(sorted.len());
    for point in sorted.iter().rev() {
        while upper.len() >= 2 {
            let turn = cross(upper[upper.len() - 2], upper[upper.len() - 1], *point);
            if !turn.is_finite() {
                return Err("convex-hull arithmetic overflowed".to_owned());
            }
            if turn > 0.0 {
                break;
            }
            upper.pop();
        }
        upper.push(*point);
    }

    lower.pop();
    upper.pop();
    lower.extend(upper);
    Ok(lower)
}

fn polygon_area(hull: &[Point2]) -> Result<f64, String> {
    if hull.len() < 3 {
        return Ok(0.0);
    }

    // Translate by the first vertex to reduce cancellation for large offsets.
    let origin = hull[0];
    let mut twice_area = 0.0;
    for index in 1..(hull.len() - 1) {
        let term = cross(origin, hull[index], hull[index + 1]);
        if !term.is_finite() {
            return Err("support-polygon area calculation overflowed".to_owned());
        }
        twice_area += term;
        if !twice_area.is_finite() {
            return Err("support-polygon area accumulation overflowed".to_owned());
        }
    }
    Ok(twice_area.abs() * 0.5)
}

fn distance_to_segment(point: Point2, start: Point2, end: Point2) -> Result<f64, String> {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length_squared = dx * dx + dy * dy;
    if !length_squared.is_finite() {
        return Err("support-edge length calculation overflowed".to_owned());
    }
    if length_squared == 0.0 {
        return Ok((point.x - start.x).hypot(point.y - start.y));
    }

    let projection = ((point.x - start.x) * dx + (point.y - start.y) * dy) / length_squared;
    if !projection.is_finite() {
        return Err("point-to-edge projection overflowed".to_owned());
    }
    let clamped = projection.clamp(0.0, 1.0);
    let nearest_x = start.x + clamped * dx;
    let nearest_y = start.y + clamped * dy;
    let distance = (point.x - nearest_x).hypot(point.y - nearest_y);
    if !distance.is_finite() {
        return Err("point-to-edge distance overflowed".to_owned());
    }
    Ok(distance)
}

fn signed_margin(point: Point2, hull: &[Point2]) -> Result<f64, String> {
    if hull.len() < 3 {
        return Err("signed margin requires a polygon with at least three vertices".to_owned());
    }

    let mut inside = true;
    let mut minimum_distance = f64::INFINITY;
    for index in 0..hull.len() {
        let start = hull[index];
        let end = hull[(index + 1) % hull.len()];
        let side = cross(start, end, point);
        if !side.is_finite() {
            return Err("point-in-polygon calculation overflowed".to_owned());
        }
        if side < 0.0 {
            inside = false;
        }
        minimum_distance = minimum_distance.min(distance_to_segment(point, start, end)?);
    }

    if !minimum_distance.is_finite() {
        return Err("support margin could not be computed".to_owned());
    }
    Ok(if inside {
        minimum_distance
    } else {
        -minimum_distance
    })
}

/// Evaluate a static gravity projection against declared support contacts.
///
/// Contact-position uncertainty is conservatively bounded by subtracting the
/// largest supplied contact radius, and centre-of-mass projection uncertainty
/// is subtracted separately. This is a simple conservative development rule,
/// not a general interval-geometry or dynamic-stability certificate.
pub fn evaluate(snapshot: &SupportSnapshot) -> SupportEvaluation {
    if let Err(error) = validate_snapshot(snapshot) {
        return invalid(snapshot, error);
    }

    if snapshot.contacts.len() < 3 {
        return SupportEvaluation {
            disposition: SupportDisposition::InsufficientContacts,
            support_hull_m: Vec::new(),
            hull_area_m2: Some(0.0),
            contact_count: snapshot.contacts.len(),
            nominal_signed_margin_m: None,
            conservative_margin_m: None,
            uncertainty_deduction_m: None,
            required_margin_m: snapshot.required_margin_m,
            error: Some("at least three declared contacts are required".to_owned()),
        };
    }

    let points: Vec<Point2> = snapshot
        .contacts
        .iter()
        .map(|contact| Point2::from_array(contact.position_m))
        .collect();

    let hull = match convex_hull(&points) {
        Ok(hull) => hull,
        Err(error) => return invalid(snapshot, error),
    };
    let area = match polygon_area(&hull) {
        Ok(area) => area,
        Err(error) => return invalid(snapshot, error),
    };

    if hull.len() < 3 || area == 0.0 {
        return SupportEvaluation {
            disposition: SupportDisposition::DegenerateSupport,
            support_hull_m: hull.iter().copied().map(Point2::to_array).collect(),
            hull_area_m2: Some(area),
            contact_count: snapshot.contacts.len(),
            nominal_signed_margin_m: None,
            conservative_margin_m: None,
            uncertainty_deduction_m: None,
            required_margin_m: snapshot.required_margin_m,
            error: Some("support contacts do not form a positive-area polygon".to_owned()),
        };
    }

    let com = Point2::from_array(snapshot.com_projection_m);
    let nominal = match signed_margin(com, &hull) {
        Ok(margin) => margin,
        Err(error) => return invalid(snapshot, error),
    };
    let maximum_contact_uncertainty = snapshot
        .contacts
        .iter()
        .map(|contact| contact.uncertainty_radius_m)
        .fold(0.0_f64, f64::max);
    let uncertainty_deduction = snapshot.com_uncertainty_m + maximum_contact_uncertainty;
    if !uncertainty_deduction.is_finite() {
        return invalid(snapshot, "combined support uncertainty overflowed");
    }
    let conservative = nominal - uncertainty_deduction;
    if !conservative.is_finite() {
        return invalid(snapshot, "conservative support margin overflowed");
    }

    let disposition = if conservative >= snapshot.required_margin_m {
        SupportDisposition::Stable
    } else if conservative >= 0.0 {
        SupportDisposition::Marginal
    } else {
        SupportDisposition::Outside
    };

    SupportEvaluation {
        disposition,
        support_hull_m: hull.iter().copied().map(Point2::to_array).collect(),
        hull_area_m2: Some(area),
        contact_count: snapshot.contacts.len(),
        nominal_signed_margin_m: Some(nominal),
        conservative_margin_m: Some(conservative),
        uncertainty_deduction_m: Some(uncertainty_deduction),
        required_margin_m: snapshot.required_margin_m,
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame() -> SupportFrame {
        SupportFrame {
            frame_id: "robot:base".to_owned(),
            plane_id: "floor:test-01".to_owned(),
            revision: 7,
        }
    }

    fn contact(foot_id: &str, position_m: [f64; 2]) -> ContactPoint {
        ContactPoint {
            foot_id: foot_id.to_owned(),
            frame_id: "robot:base".to_owned(),
            plane_id: "floor:test-01".to_owned(),
            frame_revision: 7,
            position_m,
            uncertainty_radius_m: 0.0,
        }
    }

    fn square_snapshot(com: [f64; 2]) -> SupportSnapshot {
        SupportSnapshot {
            robot_id: "quadruped:test".to_owned(),
            morphology_revision: "morphology:v1".to_owned(),
            support_frame: frame(),
            com_projection_m: com,
            com_uncertainty_m: 0.0,
            required_margin_m: 0.1,
            contacts: vec![
                contact("front-left", [-0.5, 0.3]),
                contact("front-right", [0.5, 0.3]),
                contact("rear-left", [-0.5, -0.3]),
                contact("rear-right", [0.5, -0.3]),
            ],
        }
    }

    #[test]
    fn centered_com_has_known_positive_support_margin() {
        let result = evaluate(&square_snapshot([0.0, 0.0]));
        assert_eq!(result.disposition, SupportDisposition::Stable);
        assert_eq!(result.hull_area_m2, Some(0.6));
        assert_eq!(result.nominal_signed_margin_m, Some(0.3));
        assert_eq!(result.conservative_margin_m, Some(0.3));
    }

    #[test]
    fn near_edge_com_is_marginal_when_below_required_margin() {
        let result = evaluate(&square_snapshot([0.0, 0.25]));
        assert_eq!(result.disposition, SupportDisposition::Marginal);
        assert!((result.nominal_signed_margin_m.unwrap() - 0.05).abs() < 1e-12);
    }

    #[test]
    fn uncertainty_can_reduce_nominally_stable_support_to_marginal() {
        let mut snapshot = square_snapshot([0.0, 0.2]);
        snapshot.com_uncertainty_m = 0.08;
        let result = evaluate(&snapshot);
        assert_eq!(result.disposition, SupportDisposition::Marginal);
        assert!((result.conservative_margin_m.unwrap() - 0.02).abs() < 1e-12);
    }

    #[test]
    fn outside_com_has_negative_signed_margin() {
        let result = evaluate(&square_snapshot([0.0, 0.7]));
        assert_eq!(result.disposition, SupportDisposition::Outside);
        assert!(result.nominal_signed_margin_m.unwrap() < 0.0);
    }

    #[test]
    fn three_non_collinear_contacts_form_a_triangle() {
        let mut snapshot = square_snapshot([0.0, 0.0]);
        snapshot.contacts = vec![
            contact("front", [0.0, 0.5]),
            contact("left", [-0.5, -0.5]),
            contact("right", [0.5, -0.5]),
        ];
        let result = evaluate(&snapshot);
        assert_eq!(result.disposition, SupportDisposition::Stable);
        assert_eq!(result.support_hull_m.len(), 3);
    }

    #[test]
    fn fewer_than_three_contacts_are_insufficient() {
        let mut snapshot = square_snapshot([0.0, 0.0]);
        snapshot.contacts.truncate(2);
        assert_eq!(
            evaluate(&snapshot).disposition,
            SupportDisposition::InsufficientContacts
        );
    }

    #[test]
    fn collinear_contacts_are_degenerate() {
        let mut snapshot = square_snapshot([0.0, 0.0]);
        snapshot.contacts = vec![
            contact("a", [-1.0, 0.0]),
            contact("b", [0.0, 0.0]),
            contact("c", [1.0, 0.0]),
        ];
        assert_eq!(
            evaluate(&snapshot).disposition,
            SupportDisposition::DegenerateSupport
        );
    }

    #[test]
    fn duplicate_foot_identity_is_invalid() {
        let mut snapshot = square_snapshot([0.0, 0.0]);
        snapshot.contacts[1].foot_id = snapshot.contacts[0].foot_id.clone();
        assert_eq!(
            evaluate(&snapshot).disposition,
            SupportDisposition::InvalidEvidence
        );
    }

    #[test]
    fn stale_frame_revision_is_invalid() {
        let mut snapshot = square_snapshot([0.0, 0.0]);
        snapshot.contacts[0].frame_revision += 1;
        assert_eq!(
            evaluate(&snapshot).disposition,
            SupportDisposition::InvalidEvidence
        );
    }

    #[test]
    fn non_finite_position_is_invalid() {
        let mut snapshot = square_snapshot([0.0, 0.0]);
        snapshot.contacts[0].position_m[0] = f64::NAN;
        assert_eq!(
            evaluate(&snapshot).disposition,
            SupportDisposition::InvalidEvidence
        );
    }

    #[test]
    fn negative_uncertainty_is_invalid() {
        let mut snapshot = square_snapshot([0.0, 0.0]);
        snapshot.contacts[0].uncertainty_radius_m = -0.01;
        assert_eq!(
            evaluate(&snapshot).disposition,
            SupportDisposition::InvalidEvidence
        );
    }

    #[test]
    fn input_contact_order_does_not_change_result() {
        let mut first = square_snapshot([0.1, -0.05]);
        let mut second = first.clone();
        second.contacts.reverse();
        assert_eq!(evaluate(&first), evaluate(&second));
        first.contacts.rotate_left(1);
        assert_eq!(evaluate(&first), evaluate(&second));
    }

    #[test]
    fn repeated_contact_positions_do_not_create_a_support_polygon() {
        let mut snapshot = square_snapshot([0.0, 0.0]);
        snapshot.contacts = vec![
            contact("a", [0.0, 0.0]),
            contact("b", [0.0, 0.0]),
            contact("c", [0.0, 0.0]),
        ];
        assert_eq!(
            evaluate(&snapshot).disposition,
            SupportDisposition::DegenerateSupport
        );
    }

    #[test]
    fn mixed_plane_identity_is_invalid() {
        let mut snapshot = square_snapshot([0.0, 0.0]);
        snapshot.contacts[0].plane_id = "floor:other".to_owned();
        assert_eq!(
            evaluate(&snapshot).disposition,
            SupportDisposition::InvalidEvidence
        );
    }

    #[test]
    fn non_finite_uncertainty_and_negative_threshold_are_invalid() {
        let mut snapshot = square_snapshot([0.0, 0.0]);
        snapshot.com_uncertainty_m = f64::INFINITY;
        assert_eq!(
            evaluate(&snapshot).disposition,
            SupportDisposition::InvalidEvidence
        );
        snapshot.com_uncertainty_m = 0.0;
        snapshot.required_margin_m = -0.1;
        assert_eq!(
            evaluate(&snapshot).disposition,
            SupportDisposition::InvalidEvidence
        );
    }
}

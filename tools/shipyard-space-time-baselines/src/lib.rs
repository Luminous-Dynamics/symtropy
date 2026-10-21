//! Transparent first-fit/bottom-left baseline for bounded shipyard staging layouts.
//!
//! This module is a benchmark baseline, not an optimizer or operational planner. It preserves
//! the caller's request order, tests a deterministic set of boundary-aligned placements, and
//! sends the result through the separate shipyard-space-time verifier.
//!
//! V0 supports axis-aligned rectangular blocks, finite rectangular yard bounds, half-open
//! integer-tick occupancy, and rectangular exclusion zones. It does not model irregular shapes,
//! rotation, transfer routes, lifting, crane constraints, stacking, floor bearing, or safety.

use std::collections::BTreeSet;

use shipyard_space_time::{ExclusionZone, Footprint, Placement, Rect, Scenario, TimeRange, verify};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementRequest {
    pub reservation_id: String,
    pub block_id: String,
    /// Required width in integer lattice units. Must be positive.
    pub width: i64,
    /// Required height in integer lattice units. Must be positive.
    pub height: i64,
    pub occupied: TimeRange,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BaselineErrorCode {
    EmptyRequestSet,
    InvalidYardBounds,
    EmptyReservationId,
    DuplicateReservationId,
    EmptyBlockId,
    DuplicateBlockId,
    InvalidDimensions,
    InvalidTimeRange,
    EmptyExclusionZoneId,
    DuplicateExclusionZoneId,
    InvalidExclusionZoneTime,
    InvalidExclusionZoneFootprint,
    UnsupportedExclusionGeometry,
    OutOfBoundsExclusionZone,
    NoFeasiblePlacement,
    VerifierRejected,
}

impl BaselineErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EmptyRequestSet => "EmptyRequestSet",
            Self::InvalidYardBounds => "InvalidYardBounds",
            Self::EmptyReservationId => "EmptyReservationId",
            Self::DuplicateReservationId => "DuplicateReservationId",
            Self::EmptyBlockId => "EmptyBlockId",
            Self::DuplicateBlockId => "DuplicateBlockId",
            Self::InvalidDimensions => "InvalidDimensions",
            Self::InvalidTimeRange => "InvalidTimeRange",
            Self::EmptyExclusionZoneId => "EmptyExclusionZoneId",
            Self::DuplicateExclusionZoneId => "DuplicateExclusionZoneId",
            Self::InvalidExclusionZoneTime => "InvalidExclusionZoneTime",
            Self::InvalidExclusionZoneFootprint => "InvalidExclusionZoneFootprint",
            Self::UnsupportedExclusionGeometry => "UnsupportedExclusionGeometry",
            Self::OutOfBoundsExclusionZone => "OutOfBoundsExclusionZone",
            Self::NoFeasiblePlacement => "NoFeasiblePlacement",
            Self::VerifierRejected => "VerifierRejected",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BaselineError {
    pub code: BaselineErrorCode,
    pub subject_id: String,
    pub detail: String,
}

fn add_error(
    errors: &mut Vec<BaselineError>,
    code: BaselineErrorCode,
    subject_id: impl Into<String>,
    detail: impl Into<String>,
) {
    errors.push(BaselineError {
        code,
        subject_id: subject_id.into(),
        detail: detail.into(),
    });
}

fn is_nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn request_rect(request: &PlacementRequest, min_x: i64, min_y: i64) -> Option<Rect> {
    Some(Rect {
        min_x,
        min_y,
        max_x: min_x.checked_add(request.width)?,
        max_y: min_y.checked_add(request.height)?,
    })
}

/// Generate deterministic candidate lower-left coordinates.
///
/// Boundaries of already reserved rectangles and exclusions are included on both sides of
/// each obstacle. This is a transparent bottom-left-style candidate set, not an exhaustive
/// enumeration of every integer coordinate in the yard and not a proof of global optimality.
fn candidate_coordinates(
    yard: Rect,
    request: &PlacementRequest,
    existing: &[Placement],
    zones: &[ExclusionZone],
) -> (Vec<i64>, Vec<i64>) {
    let mut xs = vec![yard.min_x];
    let mut ys = vec![yard.min_y];

    for footprint in existing
        .iter()
        .map(|placement| &placement.footprint)
        .chain(zones.iter().map(|zone| &zone.footprint))
    {
        if let Footprint::AxisAlignedRect(rect) = footprint {
            xs.extend(
                [
                    rect.min_x.checked_sub(request.width),
                    Some(rect.min_x),
                    rect.max_x.checked_sub(request.width),
                    Some(rect.max_x),
                ]
                .into_iter()
                .flatten(),
            );
            ys.extend(
                [
                    rect.min_y.checked_sub(request.height),
                    Some(rect.min_y),
                    rect.max_y.checked_sub(request.height),
                    Some(rect.max_y),
                ]
                .into_iter()
                .flatten(),
            );
        }
    }

    xs.retain(|x| *x >= yard.min_x && *x < yard.max_x);
    ys.retain(|y| *y >= yard.min_y && *y < yard.max_y);
    xs.sort_unstable();
    xs.dedup();
    ys.sort_unstable();
    ys.dedup();
    (xs, ys)
}

fn validate_inputs(
    yard: Rect,
    requests: &[PlacementRequest],
    zones: &[ExclusionZone],
) -> Vec<BaselineError> {
    let mut errors = Vec::new();
    if !yard.is_valid() {
        add_error(
            &mut errors,
            BaselineErrorCode::InvalidYardBounds,
            "yard",
            "yard bounds must have positive width and height",
        );
    }
    if requests.is_empty() {
        add_error(
            &mut errors,
            BaselineErrorCode::EmptyRequestSet,
            "yard",
            "an empty request set is not a qualified placement plan",
        );
    }

    let mut reservation_ids = BTreeSet::new();
    let mut block_ids = BTreeSet::new();
    for request in requests {
        if !is_nonempty(&request.reservation_id) {
            add_error(
                &mut errors,
                BaselineErrorCode::EmptyReservationId,
                "",
                "reservation ID must not be empty",
            );
        } else if !reservation_ids.insert(request.reservation_id.as_str()) {
            add_error(
                &mut errors,
                BaselineErrorCode::DuplicateReservationId,
                &request.reservation_id,
                "reservation IDs must be unique",
            );
        }

        if !is_nonempty(&request.block_id) {
            add_error(
                &mut errors,
                BaselineErrorCode::EmptyBlockId,
                &request.reservation_id,
                "block ID must not be empty",
            );
        } else if !block_ids.insert(request.block_id.as_str()) {
            add_error(
                &mut errors,
                BaselineErrorCode::DuplicateBlockId,
                &request.block_id,
                "each V0 request must refer to a distinct block",
            );
        }

        if request.width <= 0 || request.height <= 0 {
            add_error(
                &mut errors,
                BaselineErrorCode::InvalidDimensions,
                &request.reservation_id,
                "width and height must be positive integer lattice units",
            );
        }
        if !request.occupied.is_valid() {
            add_error(
                &mut errors,
                BaselineErrorCode::InvalidTimeRange,
                &request.reservation_id,
                "occupancy interval must be a non-empty half-open time range",
            );
        }
    }

    let mut zone_ids = BTreeSet::new();
    for zone in zones {
        if !is_nonempty(&zone.id) {
            add_error(
                &mut errors,
                BaselineErrorCode::EmptyExclusionZoneId,
                "",
                "exclusion-zone ID must not be empty",
            );
        } else if !zone_ids.insert(zone.id.as_str()) {
            add_error(
                &mut errors,
                BaselineErrorCode::DuplicateExclusionZoneId,
                &zone.id,
                "exclusion-zone IDs must be unique",
            );
        }
        if !zone.active.is_valid() {
            add_error(
                &mut errors,
                BaselineErrorCode::InvalidExclusionZoneTime,
                &zone.id,
                "exclusion-zone active interval must be non-empty",
            );
        }
        match &zone.footprint {
            Footprint::AxisAlignedRect(rect) if !rect.is_valid() => add_error(
                &mut errors,
                BaselineErrorCode::InvalidExclusionZoneFootprint,
                &zone.id,
                "exclusion-zone rectangle is invalid",
            ),
            Footprint::AxisAlignedRect(rect) if yard.is_valid() && !yard.contains(*rect) => {
                add_error(
                    &mut errors,
                    BaselineErrorCode::OutOfBoundsExclusionZone,
                    &zone.id,
                    "exclusion-zone rectangle must fit inside the declared yard",
                )
            }
            Footprint::Unsupported { .. } => add_error(
                &mut errors,
                BaselineErrorCode::UnsupportedExclusionGeometry,
                &zone.id,
                "V0 cannot plan around unsupported exclusion geometry",
            ),
            Footprint::AxisAlignedRect(_) => {}
        }
    }

    errors
}

/// Place rectangular requests using a deterministic first-fit/bottom-left candidate order.
///
/// Requests are processed in caller-provided order. Candidate positions are tried in ascending
/// (y, x) order; reservations may share floor space only when their half-open occupancy
/// intervals do not overlap. Exclusion zones consume space only while their active interval
/// overlaps the request. Any invalid input, exhausted candidate set, or verifier disagreement
/// fails the entire call without returning a partial plan.
///
/// The returned scenario contains only explicit geometry/time reservations. It is not an
/// optimized schedule, transfer route, crane/lift instruction or production authorization.
pub fn first_fit_bottom_left(
    yard_bounds: Rect,
    requests: &[PlacementRequest],
    exclusion_zones: &[ExclusionZone],
) -> Result<Scenario, Vec<BaselineError>> {
    let input_errors = validate_inputs(yard_bounds, requests, exclusion_zones);
    if !input_errors.is_empty() {
        return Err(input_errors);
    }

    let mut placements: Vec<Placement> = Vec::with_capacity(requests.len());
    for request in requests {
        let (xs, ys) = candidate_coordinates(yard_bounds, request, &placements, exclusion_zones);
        let mut selected = None;

        'candidate: for y in &ys {
            for x in &xs {
                let Some(rect) = request_rect(request, *x, *y) else {
                    continue;
                };
                if !yard_bounds.contains(rect) {
                    continue;
                }

                for prior in &placements {
                    if prior.occupied.overlaps(request.occupied)
                        && let Footprint::AxisAlignedRect(prior_rect) = &prior.footprint
                        && rect.overlaps(*prior_rect)
                    {
                        continue 'candidate;
                    }
                }

                for zone in exclusion_zones {
                    if zone.active.overlaps(request.occupied)
                        && let Footprint::AxisAlignedRect(zone_rect) = &zone.footprint
                        && rect.overlaps(*zone_rect)
                    {
                        continue 'candidate;
                    }
                }

                selected = Some(rect);
                break 'candidate;
            }
        }

        let Some(rect) = selected else {
            return Err(vec![BaselineError {
                code: BaselineErrorCode::NoFeasiblePlacement,
                subject_id: request.reservation_id.clone(),
                detail: "no candidate rectangle fits the yard and active reservations".to_owned(),
            }]);
        };

        placements.push(Placement {
            reservation_id: request.reservation_id.clone(),
            block_id: request.block_id.clone(),
            footprint: Footprint::AxisAlignedRect(rect),
            occupied: request.occupied,
        });
    }

    let scenario = Scenario {
        yard_bounds: Some(yard_bounds),
        placements,
        transfers: Vec::new(),
        exclusion_zones: exclusion_zones.to_vec(),
    };

    let report = verify(&scenario);
    if !report.is_feasible() {
        let errors = report
            .violations
            .into_iter()
            .map(|violation| BaselineError {
                code: BaselineErrorCode::VerifierRejected,
                subject_id: violation.subject_ids.join(";"),
                detail: format!("independent verifier rejected {}", violation.code.as_str()),
            })
            .collect();
        return Err(errors);
    }

    Ok(scenario)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet as CellSet;

    fn yard() -> Rect {
        Rect {
            min_x: 0,
            min_y: 0,
            max_x: 10,
            max_y: 6,
        }
    }

    fn request(
        reservation_id: &str,
        block_id: &str,
        width: i64,
        height: i64,
        start_tick: u64,
        end_tick: u64,
    ) -> PlacementRequest {
        PlacementRequest {
            reservation_id: reservation_id.to_owned(),
            block_id: block_id.to_owned(),
            width,
            height,
            occupied: TimeRange {
                start_tick,
                end_tick,
            },
        }
    }

    fn rect_of(placement: &Placement) -> Rect {
        match &placement.footprint {
            Footprint::AxisAlignedRect(rect) => *rect,
            Footprint::Unsupported { .. } => panic!("baseline must emit supported rectangles"),
        }
    }

    #[test]
    fn concurrent_rectangles_are_placed_and_independently_verifiable() {
        let requests = [
            request("r-a", "block-a", 6, 4, 0, 10),
            request("r-b", "block-b", 4, 4, 0, 10),
            request("r-c", "block-c", 4, 2, 0, 10),
        ];
        let scenario = first_fit_bottom_left(yard(), &requests, &[]).unwrap();
        assert_eq!(scenario.placements.len(), 3);
        assert!(verify(&scenario).is_feasible());

        // A small independent cell oracle checks that simultaneous placements share no cell.
        for (index, left) in scenario.placements.iter().enumerate() {
            let left_rect = rect_of(left);
            let left_cells: CellSet<(i64, i64)> = (left_rect.min_x..left_rect.max_x)
                .flat_map(|x| (left_rect.min_y..left_rect.max_y).map(move |y| (x, y)))
                .collect();
            for right in scenario.placements.iter().skip(index + 1) {
                if !left.occupied.overlaps(right.occupied) {
                    continue;
                }
                let right_rect = rect_of(right);
                let right_cells: CellSet<(i64, i64)> = (right_rect.min_x..right_rect.max_x)
                    .flat_map(|x| (right_rect.min_y..right_rect.max_y).map(move |y| (x, y)))
                    .collect();
                assert!(left_cells.is_disjoint(&right_cells));
            }
        }
    }

    #[test]
    fn non_overlapping_time_windows_can_reuse_the_same_space() {
        let requests = [
            request("r-a", "block-a", 5, 4, 0, 4),
            request("r-b", "block-b", 5, 4, 4, 8),
        ];
        let scenario = first_fit_bottom_left(yard(), &requests, &[]).unwrap();
        assert_eq!(
            rect_of(&scenario.placements[0]),
            rect_of(&scenario.placements[1])
        );
        assert!(verify(&scenario).is_feasible());
    }

    #[test]
    fn active_exclusion_zone_pushes_first_fit_to_the_next_candidate() {
        let zones = [ExclusionZone {
            id: "zone-a".to_owned(),
            footprint: Footprint::AxisAlignedRect(Rect {
                min_x: 0,
                min_y: 0,
                max_x: 5,
                max_y: 6,
            }),
            active: TimeRange {
                start_tick: 0,
                end_tick: 10,
            },
        }];
        let scenario =
            first_fit_bottom_left(yard(), &[request("r-a", "block-a", 3, 3, 0, 5)], &zones)
                .unwrap();
        assert!(matches!(
            &scenario.placements[0].footprint,
            Footprint::AxisAlignedRect(Rect { min_x: 5, .. })
        ));
        assert!(verify(&scenario).is_feasible());
    }

    #[test]
    fn full_yard_fails_instead_of_returning_a_partial_plan() {
        let requests = [
            request("r-a", "block-a", 10, 6, 0, 10),
            request("r-b", "block-b", 1, 1, 0, 10),
        ];
        let errors = first_fit_bottom_left(yard(), &requests, &[]).unwrap_err();
        assert_eq!(errors[0].code, BaselineErrorCode::NoFeasiblePlacement);
        assert_eq!(errors[0].subject_id, "r-b");
    }

    #[test]
    fn identical_inputs_produce_identical_layouts_and_errors() {
        let requests = [
            request("r-a", "block-a", 4, 3, 0, 8),
            request("r-b", "block-b", 5, 3, 0, 8),
        ];
        let left = first_fit_bottom_left(yard(), &requests, &[]).unwrap();
        let right = first_fit_bottom_left(yard(), &requests, &[]).unwrap();
        assert_eq!(left, right);
    }

    #[test]
    fn empty_requests_and_unsupported_exclusions_fail_closed() {
        assert_eq!(
            first_fit_bottom_left(yard(), &[], &[]).unwrap_err()[0].code,
            BaselineErrorCode::EmptyRequestSet
        );
        let zones = [ExclusionZone {
            id: "unknown-zone".to_owned(),
            footprint: Footprint::Unsupported {
                profile: "polygon-v1".to_owned(),
            },
            active: TimeRange {
                start_tick: 0,
                end_tick: 10,
            },
        }];
        assert_eq!(
            first_fit_bottom_left(yard(), &[request("r-a", "block-a", 1, 1, 0, 2)], &zones)
                .unwrap_err()[0]
                .code,
            BaselineErrorCode::UnsupportedExclusionGeometry
        );
    }
}

//! Independent brute-force grid oracle for the chamber packing primitive.
//!
//! This reference enumerates every integer-mm hull origin. It deliberately does
//! not reuse the production edge-coordinate candidate generator or overlap
//! predicate.

use maritime_service_chain_fleet::chamber_packing::{
    ChamberPackingResult, ChamberPlacement, ChamberSpec, PackingTermination, VesselFootprint,
    exact_chamber_packing,
};

fn reference_fits(
    chamber: &ChamberSpec,
    vessel: &VesselFootprint,
    x: u64,
    y: u64,
    placed: &[ChamberPlacement],
) -> bool {
    let clearance = chamber.clearance_mm;
    if x < clearance
        || y < clearance
        || x.checked_add(vessel.length_mm)
            .and_then(|end| end.checked_add(clearance))
            .is_none_or(|end| end > chamber.length_mm)
        || y.checked_add(vessel.beam_mm)
            .and_then(|end| end.checked_add(clearance))
            .is_none_or(|end| end > chamber.width_mm)
    {
        return false;
    }

    placed.iter().all(|other| {
        if other.chamber_id != chamber.chamber_id {
            return true;
        }
        let gap = clearance.saturating_mul(2);
        let separated = x
            .checked_add(vessel.length_mm)
            .and_then(|end| end.checked_add(gap))
            .is_some_and(|right| right <= other.x_mm)
            || other
                .x_mm
                .checked_add(other.length_mm)
                .and_then(|end| end.checked_add(gap))
                .is_some_and(|right| right <= x)
            || y
                .checked_add(vessel.beam_mm)
                .and_then(|end| end.checked_add(gap))
                .is_some_and(|top| top <= other.y_mm)
            || other
                .y_mm
                .checked_add(other.beam_mm)
                .and_then(|end| end.checked_add(gap))
                .is_some_and(|top| top <= y);
        separated
    })
}

fn reference_pack_recursive(
    chambers: &[ChamberSpec],
    vessels: &[VesselFootprint],
    vessel_index: usize,
    placed: &mut Vec<ChamberPlacement>,
) -> bool {
    if vessel_index == vessels.len() {
        return true;
    }
    let vessel = &vessels[vessel_index];
    for chamber in chambers {
        for x in chamber.clearance_mm..=chamber.length_mm {
            for y in chamber.clearance_mm..=chamber.width_mm {
                if !reference_fits(chamber, vessel, x, y, placed) {
                    continue;
                }
                placed.push(ChamberPlacement {
                    vessel_id: vessel.vessel_id.clone(),
                    chamber_id: chamber.chamber_id.clone(),
                    x_mm: x,
                    y_mm: y,
                    length_mm: vessel.length_mm,
                    beam_mm: vessel.beam_mm,
                });
                if reference_pack_recursive(
                    chambers,
                    vessels,
                    vessel_index + 1,
                    placed,
                ) {
                    return true;
                }
                placed.pop();
            }
        }
    }
    false
}

fn reference_pack(chambers: &[ChamberSpec], vessels: &[VesselFootprint]) -> bool {
    let mut placed = Vec::new();
    reference_pack_recursive(chambers, vessels, 0, &mut placed)
}

fn same_feasibility(
    actual: &ChamberPackingResult,
    expected_feasible: bool,
    label: &str,
) {
    let actual_feasible = actual.termination == PackingTermination::Feasible;
    assert_eq!(actual_feasible, expected_feasible, "{label}: {actual:?}");
    if actual.termination == PackingTermination::Infeasible {
        assert!(actual.placements.is_empty(), "{label}: partial placements leaked");
    }
    if actual.termination == PackingTermination::BudgetExhausted {
        assert!(actual.placements.is_empty(), "{label}: partial placements leaked");
    }
}

#[test]
fn exact_edge_candidate_search_matches_grid_oracle_for_all_tiny_vessel_pairs() {
    let mut checked = 0_u64;
    for chamber_length in 2..=5 {
        for chamber_width in 2..=5 {
            for clearance in 0..=1 {
                let chamber = ChamberSpec {
                    chamber_id: "chamber-1".into(),
                    length_mm: chamber_length,
                    width_mm: chamber_width,
                    clearance_mm: clearance,
                };
                if clearance * 2 >= chamber_length || clearance * 2 >= chamber_width {
                    continue;
                }
                for first_length in 1..=4 {
                    for first_beam in 1..=4 {
                        for second_length in 1..=4 {
                            for second_beam in 1..=4 {
                                let vessels = vec![
                                    VesselFootprint {
                                        vessel_id: "vessel-a".into(),
                                        length_mm: first_length,
                                        beam_mm: first_beam,
                                    },
                                    VesselFootprint {
                                        vessel_id: "vessel-b".into(),
                                        length_mm: second_length,
                                        beam_mm: second_beam,
                                    },
                                ];
                                let expected = reference_pack(
                                    std::slice::from_ref(&chamber),
                                    &vessels,
                                );
                                let actual = exact_chamber_packing(
                                    std::slice::from_ref(&chamber),
                                    &vessels,
                                    100_000,
                                )
                                .expect("tiny dimensions must be valid");
                                same_feasibility(
                                    &actual,
                                    expected,
                                    &format!(
                                        "chamber={chamber_length}x{chamber_width} c={clearance}; vessels=({first_length}x{first_beam},{second_length}x{second_beam})"
                                    ),
                                );
                                checked += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(checked >= 8_000);
}

#[test]
fn multiple_chamber_edge_search_matches_grid_oracle() {
    let chambers = vec![
        ChamberSpec {
            chamber_id: "small".into(),
            length_mm: 4,
            width_mm: 3,
            clearance_mm: 0,
        },
        ChamberSpec {
            chamber_id: "wide".into(),
            length_mm: 5,
            width_mm: 5,
            clearance_mm: 1,
        },
    ];
    let vessels = vec![
        VesselFootprint {
            vessel_id: "a".into(),
            length_mm: 4,
            beam_mm: 3,
        },
        VesselFootprint {
            vessel_id: "b".into(),
            length_mm: 3,
            beam_mm: 3,
        },
        VesselFootprint {
            vessel_id: "c".into(),
            length_mm: 2,
            beam_mm: 2,
        },
    ];
    let expected = reference_pack(&chambers, &vessels);
    let actual = exact_chamber_packing(&chambers, &vessels, 100_000).unwrap();
    same_feasibility(&actual, expected, "two-chamber reference comparison");
}

#[test]
fn complete_grid_fill_has_a_canonical_non_overlapping_witness() {
    let chamber = ChamberSpec {
        chamber_id: "c".into(),
        length_mm: 4,
        width_mm: 4,
        clearance_mm: 0,
    };
    let vessels: Vec<_> = (0..4)
        .map(|index| VesselFootprint {
            vessel_id: format!("v{index}"),
            length_mm: 2,
            beam_mm: 2,
        })
        .collect();
    let result = exact_chamber_packing(&[chamber], &vessels, 100_000).unwrap();
    assert_eq!(result.termination, PackingTermination::Feasible);
    assert_eq!(result.placements.len(), 4);
    assert!(reference_pack(
        &[ChamberSpec {
            chamber_id: "c".into(),
            length_mm: 4,
            width_mm: 4,
            clearance_mm: 0,
        }],
        &vessels
    ));
}

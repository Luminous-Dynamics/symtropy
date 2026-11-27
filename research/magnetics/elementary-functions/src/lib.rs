// SPDX-License-Identifier: MIT
#![allow(clippy::float_cmp)] // Exact mathematical boundary assertions are intentional.

//! Qualification probe for scioxide-validated elementary interval functions.
//!
//! This crate deliberately preserves the candidate's range pieces, domain
//! status and gap-filling flag. It is not yet a production Symtropy backend.

use scioxide_validated::{DecoratedInterval, ElementaryEnclosure, EnclosureStatus};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnclosureMetadata {
    pub status: EnclosureStatus,
    pub piece_count: usize,
    pub gaps_filled: bool,
}

pub fn metadata(enclosure: &ElementaryEnclosure<f64>) -> EnclosureMetadata {
    EnclosureMetadata {
        status: enclosure.status(),
        piece_count: enclosure.pieces().len(),
        gaps_filled: enclosure.gaps_filled(),
    }
}

fn interval(lower: f64, upper: f64) -> Result<DecoratedInterval<f64>, String> {
    DecoratedInterval::new(lower, upper).map_err(|error| format!("{error:?}"))
}

pub fn enclose_sin(
    lower: f64,
    upper: f64,
) -> Result<ElementaryEnclosure<f64>, String> {
    interval(lower, upper)?
        .sin()
        .map_err(|error| format!("{error:?}"))
}

pub fn enclose_cos(
    lower: f64,
    upper: f64,
) -> Result<ElementaryEnclosure<f64>, String> {
    interval(lower, upper)?
        .cos()
        .map_err(|error| format!("{error:?}"))
}

pub fn enclose_tan(
    lower: f64,
    upper: f64,
) -> Result<ElementaryEnclosure<f64>, String> {
    interval(lower, upper)?
        .tan()
        .map_err(|error| format!("{error:?}"))
}

pub fn enclose_asin(
    lower: f64,
    upper: f64,
) -> Result<ElementaryEnclosure<f64>, String> {
    interval(lower, upper)?
        .asin()
        .map_err(|error| format!("{error:?}"))
}

pub fn enclose_atan2(
    y_lower: f64,
    y_upper: f64,
    x_lower: f64,
    x_upper: f64,
) -> Result<ElementaryEnclosure<f64>, String> {
    interval(y_lower, y_upper)?
        .atan2(&interval(x_lower, x_upper)?)
        .map_err(|error| format!("{error:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_interval_encloses_the_interior_positive_extremum() {
        let enclosure = enclose_sin(1.5, 1.6).expect("finite bounded sine interval");
        assert_eq!(enclosure.status(), EnclosureStatus::Defined);
        assert!(enclosure.contains(1.0));
        assert!(!enclosure.gaps_filled());
    }

    #[test]
    fn cosine_interval_encloses_the_zero_argument_extremum() {
        let enclosure = enclose_cos(-0.1, 0.1).expect("finite bounded cosine interval");
        assert_eq!(enclosure.status(), EnclosureStatus::Defined);
        assert!(enclosure.contains(1.0));
    }

    #[test]
    fn tangent_pole_is_preserved_as_a_two_piece_partial_image() {
        let enclosure = enclose_tan(1.0, 2.0).expect("finite bounded tangent interval");
        assert_eq!(enclosure.status(), EnclosureStatus::PartiallyDefined);
        assert_eq!(enclosure.pieces().len(), 2);
        assert!(!enclosure.gaps_filled());
    }

    #[test]
    fn atan2_negative_axis_crossing_keeps_both_angle_pieces() {
        let enclosure =
            enclose_atan2(-0.1, 0.1, -2.0, -1.0).expect("box avoids atan2 origin");
        assert_eq!(enclosure.status(), EnclosureStatus::Defined);
        assert_eq!(enclosure.pieces().len(), 2);
        assert!(enclosure.contains(3.1));
        assert!(enclosure.contains(-3.1));
    }

    #[test]
    fn atan2_origin_containing_box_is_not_labelled_fully_defined() {
        let enclosure =
            enclose_atan2(-1.0, 1.0, -1.0, 1.0).expect("bounded atan2 input box");
        assert_ne!(enclosure.status(), EnclosureStatus::Defined);
        assert!(
            matches!(
                enclosure.status(),
                EnclosureStatus::PartiallyDefined | EnclosureStatus::Inconclusive
            )
        );
    }

    #[test]
    fn asin_partial_domain_status_is_retained() {
        let enclosure = enclose_asin(-2.0, 0.5).expect("partially valid inverse-sine domain");
        assert_eq!(enclosure.status(), EnclosureStatus::PartiallyDefined);
        assert!(enclosure.contains(0.5235987755982988));
    }

    #[test]
    fn large_angle_uses_the_candidates_bounded_argument_reduction() {
        let enclosure =
            enclose_sin(1.0e300, 1.0e300).expect("finite point angle in the f64 domain");
        assert_eq!(enclosure.status(), EnclosureStatus::Defined);
        assert_eq!(enclosure.pieces().len(), 1);
        assert!(!enclosure.gaps_filled());
    }

    #[test]
    fn nonfinite_interval_endpoints_fail_before_elementary_evaluation() {
        assert!(enclose_sin(f64::NEG_INFINITY, 0.0).is_err());
        assert!(enclose_cos(0.0, f64::INFINITY).is_err());
    }

    #[test]
    fn metadata_preserves_status_piece_count_and_gap_fill_information() {
        let enclosure = enclose_tan(1.0, 2.0).expect("pole-crossing tangent interval");
        let report = metadata(&enclosure);
        assert_eq!(report.status, EnclosureStatus::PartiallyDefined);
        assert_eq!(report.piece_count, 2);
        assert!(!report.gaps_filled);
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later

//! EHD PIV cluster-conformal calibration and field-functional uncertainty.
//!
//! This crate composes five typed paths:
//! 1. planar whole-acquisition conformal calibration;
//! 2. three-component whole-acquisition conformal calibration;
//! 3. propagation of field covariance to mass/momentum functionals;
//! 4. conformal calibration of integrated functionals against independent
//!    reference evidence; and
//! 5. compact-support spatial covariance for overlapping PIV windows.
//!
//! Neither module processes raw images or establishes experimental truth.

pub mod conformal;
pub mod conformal_3d;
pub mod functional;
pub mod functional_conformal;
pub mod spatial_correlation;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FlowRegion {
    Bulk,
    NearWall,
    ShearLayer,
    Wake,
}

impl FlowRegion {
    pub const ALL: [Self; 4] = [
        Self::Bulk,
        Self::NearWall,
        Self::ShearLayer,
        Self::Wake,
    ];
}

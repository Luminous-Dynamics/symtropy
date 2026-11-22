//! EHD PIV cluster-conformal calibration and field-functional uncertainty.
//!
//! This crate composes two distinct ideas:
//! 1. whole-acquisition conformal calibration using a maximum nonconformity
//!    score per predeclared, exchangeable field cluster; and
//! 2. covariance propagation from a velocity field to mass and momentum flux,
//!    retaining shared spatially correlated nuisance modes; and
//! 3. separate conformal calibration of those integrated functionals against
//!    independent reference evidence.
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

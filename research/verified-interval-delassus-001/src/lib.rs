// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Outward-rounded derivation and admission of Delassus coefficients.
//!
//! The crate derives B = M^-1 J^T and W = J M^-1 J^T from the same interval
//! inputs. Caller-supplied B and W are never accepted as authoritative.

#![forbid(unsafe_code)]

pub mod constraint_basis;
pub mod delassus;

pub use constraint_basis::{
    ConstraintBasisErrorV1, ConstraintBasisStatusV1,
    IntervalConstraintBasisCertificateV1,
    certify_interval_constraint_basis_covariance,
    verify_interval_constraint_basis_certificate,
};
pub use delassus::{
    DerivedConstraintCoefficientsV1, IntervalMatrix, VerifiedDelassusErrorV1,
    VerifiedDelassusProfileV1, VerifiedDelassusStatusV1, certify_delassus_coefficients,
    point_interval_jacobian, point_interval_matrix,
};

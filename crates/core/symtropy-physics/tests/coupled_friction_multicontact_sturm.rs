//! Exact multi-contact regime-stratum tests.
//!
//! This integration target compiles the production library without enabling
//! its large inherited unit-test module tree. The certificate and shared exact
//! arithmetic support are still compiled and executed as ordinary Rust tests.

#[path = "../src/coupled_friction_sturm_support.rs"]
mod coupled_friction_sturm_support;

#[path = "../src/coupled_friction_multicontact_sturm.rs"]
mod multicontact;

//! Compatibility re-export of the renderer-independent motion contract.
//!
//! The actual API and its validation live in the lightweight `symtropy-motion-contract`
//! crate so cognition/planning clients do not need to import the Bevy integration.

pub use symtropy_motion_contract::*;

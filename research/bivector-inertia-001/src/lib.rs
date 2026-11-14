#![forbid(unsafe_code)]
#![deny(warnings)]

// Make the production module's public crate path resolve to this isolated
// harness while including the exact Symtropy math source files by path.
extern crate self as symtropy_math;

#[path = "../../../crates/core/symtropy-math/src/bivector.rs"]
pub mod bivector;
#[path = "../../../crates/core/symtropy-math/src/point.rs"]
pub mod point;
#[path = "../../../crates/core/symtropy-math/src/rotor.rs"]
pub mod rotor;

pub use bivector::Bivector;
pub use rotor::Rotor;

#[path = "../../../crates/core/symtropy-physics/src/rotational_inertia.rs"]
pub mod rotational_inertia;

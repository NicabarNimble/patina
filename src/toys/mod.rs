//! Toy capability boundary surface.
//!
//! Phase-A facade for canonical toy ownership and discoverability.

pub mod catalog;
pub mod connector;
pub mod http;
pub mod ingress;
pub mod lake;
pub mod query;

pub use crate::mother::{GrantedIngressSource, GrantedToys, Toy};

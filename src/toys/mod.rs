//! Toy capability boundary surface.
//!
//! Phase-A facade for canonical toy ownership and discoverability.

pub mod catalog;
pub mod connector;
pub mod github;
pub mod http;
pub mod ingress;
pub mod lake;
pub mod query;
pub mod session;

pub use crate::mother::{GrantedIngressSource, GrantedToys, Toy};

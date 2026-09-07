//! Static ADC linearity audit: starter scaffold; see docs/SPEC.md.
#![forbid(unsafe_code)]

pub mod analysis;
pub mod cli;
pub mod config;
pub mod input;
pub mod model;
pub mod report;
pub mod svg;
pub mod synthetic;
pub mod transitions;

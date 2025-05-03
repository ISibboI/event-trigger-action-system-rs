//! # Event Trigger Action System (ETAS)
//!
//! The core type is [CompiledTriggers], which provides functions to execute events and to receive actions.

#![warn(missing_docs)]
#![deny(clippy::mod_module_files)]

mod conditions;
mod constructors;
#[cfg(test)]
mod tests;
mod triggers;

pub use crate::conditions::{CompiledTriggerCondition, TriggerCondition};
pub use crate::constructors::{
    and, any_n, eq, event_count, geq, gt, leq, lt, never, none, or, sequence,
};
pub use crate::triggers::{
    CompiledTrigger, CompiledTriggers, Trigger, TriggerAction, TriggerEvent, TriggerHandle,
    TriggerIdentifier, Triggers,
};

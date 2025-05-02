//! # Event Trigger Action System (ETAS)
//!
//! The core type is [CompiledTriggers], which provides functions to execute events and to receive actions.

#![warn(missing_docs)]

mod conditions;
mod constructors;
mod triggers;

pub use crate::conditions::{CompiledTriggerCondition, TriggerCondition, TriggerConditionUpdate};
pub use crate::constructors::{and, any_n, event_count, geq, never, none, or, sequence};
pub use crate::triggers::{
    CompiledTrigger, CompiledTriggers, Trigger, TriggerAction, TriggerEvent, TriggerHandle,
    TriggerIdentifier, Triggers,
};

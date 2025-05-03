use std::cmp::Ordering;

use crate::triggers::TriggerIdentifier;
use crate::{TriggerAction, TriggerEvent};

impl TriggerAction for () {}

impl TriggerIdentifier for () {}

impl TriggerEvent for () {
    type Action = ();
    type Identifier = ();

    fn identifier(&self) -> Self::Identifier {}

    fn partial_cmp_progress(&self, _other: &Self, ordering: Ordering) -> Option<f64> {
        Some(if ordering == Ordering::Equal {
            1.0
        } else {
            0.0
        })
    }
}

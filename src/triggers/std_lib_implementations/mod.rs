use crate::triggers::TriggerIdentifier;
use crate::{TriggerAction, TriggerEvent};

impl TriggerAction for () {}

impl TriggerIdentifier for () {}

impl TriggerEvent for () {
    type Action = ();
    type Identifier = ();

    fn identifier(&self) -> Self::Identifier {}

    fn value_geq(&self, _other: &Self) -> Option<bool> {
        Some(true)
    }

    fn value_geq_progress(&self, _other: &Self) -> Option<f64> {
        Some(1.0)
    }
}

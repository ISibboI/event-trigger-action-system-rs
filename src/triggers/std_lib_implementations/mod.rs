use crate::{TriggerAction, TriggerEvent};

impl TriggerAction for () {}

impl TriggerEvent for () {
    type Action = ();
}

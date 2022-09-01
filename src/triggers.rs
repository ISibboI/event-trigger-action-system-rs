use std::collections::BTreeSet;
use std::fmt::Debug;
use btreemultimap_value_ord::BTreeMultiMap;
use crate::conditions::{TriggerCondition, TriggerConditionUpdate};

#[derive(Debug)]
pub struct Triggers<Event: TriggerEvent> {
    triggers: Vec<Trigger<Event>>,
    subscriptions: BTreeMultiMap<Event, usize>,
}

#[derive(Debug)]
pub struct Trigger<Event: TriggerEvent> {
    condition: TriggerCondition<Event>,
    actions: Option<Vec<Event::Action>>,
}
#[derive(
    Debug, Clone, Copy, Default, Eq, PartialEq, Hash, Ord, PartialOrd,
)]
pub struct TriggerIdentifier(usize);

pub trait TriggerAction: Debug {}

pub trait TriggerEvent: From<Self::Action> + Ord + Clone {
    type Action: TriggerAction;
}

impl<Event: TriggerEvent> Triggers<Event> {
    pub fn new(triggers: Vec<Trigger<Event>>) -> Self {
        let subscriptions = triggers.iter().enumerate().map(|(id, trigger)| trigger.subscriptions().into_iter().map(move |event_type| (event_type, id))).flatten().collect();
        Self {
            triggers,
            subscriptions,
        }
    }

    pub fn execute_event(&mut self, event: &Event) -> Vec<Event::Action> {
        let mut result = Vec::new();
        let trigger_indices: Vec<_> = self.subscriptions.get(event).unwrap_or(&BTreeSet::new()).iter().copied().collect();
        for trigger_index in trigger_indices {
            let trigger = &mut self.triggers[trigger_index];
            let (mut actions, trigger_condition_updates) = trigger.execute_event(event);
            result.append(&mut actions);

            for trigger_condition_update in trigger_condition_updates {
                match trigger_condition_update {
                    TriggerConditionUpdate::Subscribe(event) => {
                        self.subscriptions.insert(event, trigger_index);
                    }
                    TriggerConditionUpdate::Unsubscribe(event) => {
                        // TODO also removes subscriptions if a different part of a condition still needs them
                        self.subscriptions.remove_key_value(&event, &trigger_index);
                    }
                }
            }
        }
        result
    }
}

impl<Event: TriggerEvent> Trigger<Event> {
    pub fn new(condition: TriggerCondition<Event>, actions: Vec<Event::Action>) -> Self {
        Self {
            condition,
            actions: Some(actions),
        }
    }

    pub fn subscriptions(&self) -> Vec<Event> {
        self.condition.subscriptions()
    }

    pub fn execute_event(&mut self, event: &Event) -> (Vec<Event::Action>, Vec<TriggerConditionUpdate<Event>>) {
        let (trigger_condition_updates, result, _) = self.condition.execute_event(event);
        if result {
            (self.actions.take().unwrap(), trigger_condition_updates)
        } else {
            (Default::default(), trigger_condition_updates)
        }
    }

    pub fn progress(&self) -> (f64, f64) {
        (self.condition.current_progress(), self.condition.required_progress())
    }
}

impl From<usize> for TriggerIdentifier {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

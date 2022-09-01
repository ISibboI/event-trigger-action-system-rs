use crate::conditions::{TriggerCondition, TriggerConditionUpdate};
use btreemultimap_value_ord::BTreeMultiMap;
use std::collections::{BTreeMap, VecDeque};
use std::fmt::Debug;

mod std_lib_implementations;

#[derive(Debug)]
pub struct Triggers<Event: TriggerEvent> {
    trigger_system: TriggerSystem<Event>,
    action_queue: VecDeque<Event::Action>,
}

#[derive(Debug)]
struct TriggerSystem<Event: TriggerEvent> {
    triggers: Vec<Trigger<Event>>,
    subscriptions: BTreeMultiMap<Event, usize>,
}

#[derive(Debug)]
pub struct Trigger<Event: TriggerEvent> {
    condition: TriggerCondition<Event>,
    actions: Option<Vec<Event::Action>>,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct TriggerIdentifier(usize);

pub trait TriggerAction: Debug + Clone {}

pub trait TriggerEvent: From<Self::Action> + Ord + Clone {
    type Action: TriggerAction;
}

impl<Event: TriggerEvent> Triggers<Event> {
    pub fn new(mut triggers: Vec<Trigger<Event>>) -> Self {
        let mut initial_actions = Vec::new();
        let subscriptions = triggers
            .iter_mut()
            .enumerate()
            .flat_map(|(id, trigger)| {
                let subscriptions = trigger.subscriptions();
                assert_eq!(subscriptions.is_empty(), trigger.completed());
                if trigger.completed() {
                    initial_actions.append(&mut trigger.consume_actions());
                }
                subscriptions
                    .into_iter()
                    .map(move |event_type| (event_type, id))
            })
            .collect();
        let mut trigger_system = TriggerSystem {
            triggers,
            subscriptions,
        };

        let mut i = 0;
        while i < initial_actions.len() {
            initial_actions.append(
                &mut trigger_system.execute_event(&Event::from(initial_actions[i].clone())),
            );
            i += 1;
        }

        Self {
            trigger_system,
            action_queue: initial_actions.into_iter().collect(),
        }
    }

    pub fn execute_event(&mut self, event: &Event) {
        self.action_queue
            .extend(self.trigger_system.execute_event(event).into_iter());
    }

    pub fn consume_action(&mut self) -> Option<Event::Action> {
        self.action_queue.pop_front()
    }
}

impl<Event: TriggerEvent> TriggerSystem<Event> {
    fn execute_event(&mut self, event: &Event) -> Vec<Event::Action> {
        let mut all_actions = Vec::new();
        let trigger_indices: Vec<_> = self
            .subscriptions
            .get(event)
            .unwrap_or(&BTreeMap::new())
            .keys()
            .copied()
            .collect();
        for trigger_index in trigger_indices {
            let trigger = &mut self.triggers[trigger_index];
            let (mut actions, trigger_condition_updates) = trigger.execute_event(event);
            all_actions.append(&mut actions);

            for trigger_condition_update in trigger_condition_updates {
                match trigger_condition_update {
                    TriggerConditionUpdate::Subscribe(event) => {
                        self.subscriptions.insert(event, trigger_index);
                    }
                    TriggerConditionUpdate::Unsubscribe(event) => {
                        self.subscriptions.remove_key_value(&event, &trigger_index);
                    }
                }
            }
        }

        let mut i = 0;
        while i < all_actions.len() {
            all_actions.append(&mut self.execute_event(&Event::from(all_actions[i].clone())));
            i += 1;
        }

        all_actions
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

    pub fn execute_event(
        &mut self,
        event: &Event,
    ) -> (Vec<Event::Action>, Vec<TriggerConditionUpdate<Event>>) {
        let (trigger_condition_updates, result, _) = self.condition.execute_event(event);
        if result {
            (self.actions.take().unwrap(), trigger_condition_updates)
        } else {
            (Default::default(), trigger_condition_updates)
        }
    }

    pub fn progress(&self) -> (f64, f64) {
        (
            self.condition.current_progress(),
            self.condition.required_progress(),
        )
    }

    pub fn condition(&self) -> &TriggerCondition<Event> {
        &self.condition
    }

    pub fn actions(&self) -> &[Event::Action] {
        self.actions.as_deref().unwrap_or(&[])
    }

    pub fn completed(&self) -> bool {
        self.condition.completed()
    }

    fn consume_actions(&mut self) -> Vec<Event::Action> {
        self.actions.take().unwrap()
    }
}

impl From<usize> for TriggerIdentifier {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

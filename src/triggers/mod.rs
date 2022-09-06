use crate::conditions::{CompiledTriggerCondition, TriggerConditionUpdate};
use crate::TriggerCondition;
use btreemultimap_value_ord::BTreeMultiMap;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::fmt::Debug;

mod std_lib_implementations;

#[derive(Debug, Clone)]
pub struct Triggers<Event, Action> {
    triggers: Vec<Trigger<Event, Action>>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CompiledTriggers<Event: TriggerEvent> {
    trigger_system: TriggerSystem<Event>,
    action_queue: VecDeque<Event::Action>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
struct TriggerSystem<Event: TriggerEvent> {
    triggers: Vec<CompiledTrigger<Event>>,
    subscriptions: BTreeMultiMap<Event::Identifier, usize>,
}

#[derive(Debug, Clone)]
pub struct Trigger<Event, Action> {
    pub id_str: String,
    condition: TriggerCondition<Event>,
    actions: Vec<Action>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CompiledTrigger<Event: TriggerEvent> {
    pub id_str: String,
    condition: CompiledTriggerCondition<Event>,
    actions: Option<Vec<Event::Action>>,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TriggerHandle(usize);

pub trait TriggerAction: Debug + Clone {}

pub trait TriggerIdentifier: Debug + Ord + Clone {}

#[cfg(not(feature = "serde"))]
pub trait TriggerEvent: From<Self::Action> {
    type Action: TriggerAction;
    type Identifier: TriggerIdentifier;

    fn identifier(&self) -> Self::Identifier;

    fn value_geq(&self, other: &Self) -> Option<bool>;

    /// Returns a number between 0.0 and 1.0 indicating how close the condition `value_geq` is to being fulfilled.
    /// Except if the events are not compatible, then `None` is returned.
    fn value_geq_progress(&self, other: &Self) -> Option<f64>;
}

#[cfg(feature = "serde")]
pub trait TriggerEvent: From<Self::Action> {
    type Action: TriggerAction + Serialize + for<'de> Deserialize<'de>;
    type Identifier: TriggerIdentifier + Serialize + for<'de> Deserialize<'de>;

    fn identifier(&self) -> Self::Identifier;

    fn value_geq(&self, other: &Self) -> Option<bool>;

    /// Returns a number between 0.0 and 1.0 indicating how close the condition `value_geq` is to being fulfilled.
    /// Except if the events are not compatible, then `None` is returned.
    fn value_geq_progress(&self, other: &Self) -> Option<f64>;
}

impl<Event, Action> Triggers<Event, Action> {
    pub fn new(triggers: Vec<Trigger<Event, Action>>) -> Self {
        Self { triggers }
    }

    pub fn compile<
        EventCompiler: Fn(Event) -> CompiledEvent,
        CompiledEvent: TriggerEvent,
        ActionCompiler: Fn(Action) -> CompiledEvent::Action,
    >(
        self,
        event_compiler: &EventCompiler,
        action_compiler: &ActionCompiler,
    ) -> CompiledTriggers<CompiledEvent> {
        CompiledTriggers::new(
            self.triggers
                .into_iter()
                .map(|trigger| trigger.compile(event_compiler, action_compiler))
                .collect(),
        )
    }
}

impl<Event: TriggerEvent> CompiledTriggers<Event> {
    pub fn new(mut triggers: Vec<CompiledTrigger<Event>>) -> Self {
        let mut initial_actions = Vec::new();
        let subscriptions = triggers
            .iter_mut()
            .enumerate()
            .flat_map(|(id, trigger)| {
                let subscriptions = trigger.subscriptions();
                if trigger.completed() {
                    initial_actions.append(&mut trigger.consume_actions());
                }
                subscriptions
                    .into_iter()
                    .map(move |identifier| (identifier, id))
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

    pub fn execute_events<'events>(&mut self, events: impl IntoIterator<Item = &'events Event>)
    where
        Event: 'events,
    {
        events
            .into_iter()
            .for_each(|event| self.execute_event(event));
    }

    pub fn execute_owned_events(&mut self, events: impl IntoIterator<Item = Event>) {
        events
            .into_iter()
            .for_each(|event| self.execute_event(&event));
    }

    pub fn consume_action(&mut self) -> Option<Event::Action> {
        self.action_queue.pop_front()
    }

    pub fn consume_all_actions(&mut self) -> impl '_ + Iterator<Item = Event::Action> {
        self.action_queue.drain(0..self.action_queue.len())
    }

    pub fn progress(&self, handle: TriggerHandle) -> Option<(f64, f64)> {
        self.trigger_system
            .triggers
            .get(handle.0)
            .map(|trigger| trigger.progress())
    }
}

impl<Event: TriggerEvent> TriggerSystem<Event> {
    fn execute_event(&mut self, event: &Event) -> Vec<Event::Action> {
        let mut all_actions = Vec::new();
        let identifier = event.identifier();
        let trigger_indices: Vec<_> = self
            .subscriptions
            .get(&identifier)
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
                    TriggerConditionUpdate::Subscribe(identifier) => {
                        self.subscriptions.insert(identifier.clone(), trigger_index);
                    }
                    TriggerConditionUpdate::Unsubscribe(identifier) => {
                        self.subscriptions
                            .remove_key_value(&identifier, &trigger_index);
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

impl<Event, Action> Trigger<Event, Action> {
    pub fn new(id_str: String, condition: TriggerCondition<Event>, actions: Vec<Action>) -> Self {
        Self {
            id_str,
            condition,
            actions,
        }
    }

    pub fn compile<
        EventCompiler: Fn(Event) -> CompiledEvent,
        CompiledEvent: TriggerEvent,
        ActionCompiler: Fn(Action) -> CompiledEvent::Action,
    >(
        self,
        event_compiler: &EventCompiler,
        action_compiler: &ActionCompiler,
    ) -> CompiledTrigger<CompiledEvent> {
        CompiledTrigger {
            id_str: self.id_str,
            condition: self.condition.compile(event_compiler),
            actions: Some(self.actions.into_iter().map(action_compiler).collect()),
        }
    }
}

impl<Event: TriggerEvent> CompiledTrigger<Event> {
    pub fn new(
        id_str: String,
        condition: CompiledTriggerCondition<Event>,
        actions: Vec<Event::Action>,
    ) -> Self {
        Self {
            id_str,
            condition,
            actions: Some(actions),
        }
    }

    pub fn subscriptions(&self) -> Vec<Event::Identifier> {
        self.condition.subscriptions()
    }

    pub fn execute_event(
        &mut self,
        event: &Event,
    ) -> (
        Vec<Event::Action>,
        Vec<TriggerConditionUpdate<Event::Identifier>>,
    ) {
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

    pub fn condition(&self) -> &CompiledTriggerCondition<Event> {
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

impl From<usize> for TriggerHandle {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

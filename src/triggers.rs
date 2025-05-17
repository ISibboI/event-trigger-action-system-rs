use crate::TriggerCondition;
use crate::conditions::{CompiledTriggerCondition, TriggerConditionUpdate};
use btreemultimap_value_ord::BTreeMultiMap;
use conditional_serde::ConditionalSerde;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, VecDeque};
use std::fmt::Debug;

mod std_lib_implementations;

/// A raw collection of triggers.
#[derive(Debug, Clone)]
pub struct Triggers<Event, Action> {
    triggers: Vec<Trigger<Event, Action>>,
}

/// A compiled collection of triggers.
///
/// This is the central type for using the event trigger action system.
/// Execute events via [`Self::execute_event`], [`Self::execute_events`] and [`Self::execute_owned_events`], and collect actions via [`Self::consume_action`] and [`Self::consume_all_actions`].
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

/// A raw trigger.
#[derive(Debug, Clone)]
pub struct Trigger<Event, Action> {
    /// A unique identifier of the trigger.
    pub id_str: String,
    /// The condition for the trigger to trigger.
    pub condition: TriggerCondition<Event>,
    /// The actions the trigger executes when triggered.
    pub actions: Vec<Action>,
}

/// A compiled trigger.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CompiledTrigger<Event: TriggerEvent> {
    /// A unique identifier of the trigger.
    pub id_str: String,
    condition: CompiledTriggerCondition<Event>,
    actions: Option<Vec<Event::Action>>,
}

/// A handle of a trigger.
///
/// This allows to identify a trigger without worrying about lifetimes.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TriggerHandle(usize);

/// A trigger action.
pub trait TriggerAction: Debug + Clone {}

/// An identifier of a trigger.
///
/// This is used to compare triggers while ignoring their progress towards fulfilling a comparison.
/// Hence, the identifier should ignore all data that is used to make a comparison between [`TriggerEvents`](TriggerEvent),
/// but keep all data that is used to decide comparability between [`TriggerEvents`](TriggerEvent).
///
/// Formally, the following invariant must be fulfilled: if [`partial_cmp`](PartialOrd::partial_cmp) between two trigger events returns `None`,
/// then their identifiers must be different, and vice versa.
///
/// This type should be cheap to clone.
pub trait TriggerEventIdentifier: Debug + Ord + Clone {}

/// A trigger event.
pub trait TriggerEvent: From<Self::Action> + PartialOrd {
    /// The action type used by the trigger event.
    type Action: TriggerAction + ConditionalSerde;

    /// The identifier of a trigger event.
    ///
    /// See [`TriggerIdentifier`] for details.
    type Identifier: TriggerEventIdentifier + ConditionalSerde;

    /// Returns the identifier of this trigger event.
    fn identifier(&self) -> Self::Identifier;

    /// Returns a number between 0.0 and 1.0 indicating how close the ordering of this and other is to the target ordering.
    /// If the events are not ordered, then `None` is returned.
    fn partial_cmp_progress(&self, other: &Self, target_ordering: Ordering) -> Option<f64>;
}

impl<Event, Action> Triggers<Event, Action> {
    /// Create a new raw triggers instance.
    pub fn new(triggers: Vec<Trigger<Event, Action>>) -> Self {
        Self { triggers }
    }

    /// Compile the raw triggers.
    ///
    /// Events are compiled by the event compiler, and actions are compiled by the action compiler.
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
    pub(crate) fn new(mut triggers: Vec<CompiledTrigger<Event>>) -> Self {
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

    /// Execute the given event.
    ///
    /// The event is executed right away, and all resulting actions are stored in an internal action queue,
    /// waiting to be retrieved via [`Self::consume_action`] or [`Self::consume_all_actions`].
    pub fn execute_event(&mut self, event: &Event) {
        self.action_queue
            .extend(self.trigger_system.execute_event(event));
    }

    /// Execute the given events.
    ///
    /// The event is executed right away, and all resulting actions are stored in an internal action queue,
    /// waiting to be retrieved via [`Self::consume_action`] or [`Self::consume_all_actions`].
    pub fn execute_events<'events>(&mut self, events: impl IntoIterator<Item = &'events Event>)
    where
        Event: 'events,
    {
        events
            .into_iter()
            .for_each(|event| self.execute_event(event));
    }

    /// Execute the given owned events.
    ///
    /// The event is executed right away, and all resulting actions are stored in an internal action queue,
    /// waiting to be retrieved via [`Self::consume_action`] or [`Self::consume_all_actions`].
    ///
    /// This method is no different from [`Self::execute_events`], except that it drops the given events after execution.
    pub fn execute_owned_events(&mut self, events: impl IntoIterator<Item = Event>) {
        events
            .into_iter()
            .for_each(|event| self.execute_event(&event));
    }

    /// Consume an action from the action queue, if there is one.
    pub fn consume_action(&mut self) -> Option<Event::Action> {
        self.action_queue.pop_front()
    }

    /// Consume all action from the action queue.
    ///
    /// If the returned iterator is dropped before all actions are consumed, the remaining actions are dropped quietly.
    pub fn consume_all_actions(&mut self) -> impl '_ + Iterator<Item = Event::Action> {
        self.action_queue.drain(0..self.action_queue.len())
    }

    /// Returns the progress of the given trigger as `(current_progress, required_progress)`.
    ///
    /// When `current_progress` reaches `required_progress`, then the trigger triggers.
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
    /// Creates a new raw trigger.
    pub fn new(id_str: String, condition: TriggerCondition<Event>, actions: Vec<Action>) -> Self {
        Self {
            id_str,
            condition,
            actions,
        }
    }

    /// Compiles this trigger.
    ///
    /// Events are compiled by the event compiler, and actions are compiled by the action compiler.
    pub fn compile<
        EventCompiler: Fn(Event) -> CompiledEvent,
        CompiledEvent: TriggerEvent,
        ActionCompiler: Fn(Action) -> CompiledEvent::Action,
    >(
        self,
        event_compiler: &EventCompiler,
        action_compiler: &ActionCompiler,
    ) -> CompiledTrigger<CompiledEvent> {
        CompiledTrigger::new(
            self.id_str,
            self.condition.compile(event_compiler),
            self.actions.into_iter().map(action_compiler).collect(),
        )
    }
}

impl<Event: TriggerEvent> CompiledTrigger<Event> {
    pub(crate) fn new(
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

    pub(crate) fn subscriptions(&self) -> Vec<Event::Identifier> {
        self.condition.subscriptions()
    }

    pub(crate) fn execute_event(
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

    pub(crate) fn progress(&self) -> (f64, f64) {
        (
            self.condition.current_progress(),
            self.condition.required_progress(),
        )
    }

    /// Returns the trigger condition of this trigger.
    #[allow(dead_code)]
    pub(crate) fn condition(&self) -> &CompiledTriggerCondition<Event> {
        &self.condition
    }

    /// Returns the actions of this trigger.
    #[allow(dead_code)]
    pub(crate) fn actions(&self) -> &[Event::Action] {
        self.actions.as_deref().unwrap_or(&[])
    }

    pub(crate) fn completed(&self) -> bool {
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

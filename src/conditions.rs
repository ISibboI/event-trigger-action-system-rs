use crate::triggers::TriggerEvent;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// The (uncompiled) trigger conditions for events.
///
/// Each condition triggers at most once.
#[derive(Debug, Clone)]
pub enum TriggerCondition<Event> {
    /// No trigger condition, this condition is always fulfilled.
    None,

    /// The unfulfillable trigger condition. This condition is never fulfilled.
    Never,

    /// Trigger after a certain number of events have been received.
    EventCount {
        /// The event to count.
        event: Event,
        /// The amount of times this event needs to be received for the condition to trigger.
        required: usize,
    },

    Geq {
        event: Event,
    },

    /// Trigger when the given conditions have been fulfilled in sequence.
    ///
    /// This works by having a pointer to the current active condition in the sequence.
    /// Whenever a condition is fulfilled, the pointer gets moved to the right.
    /// If it has visited all conditions, then this condition triggers.
    Sequence {
        /// The sequence of conditions.
        conditions: Vec<TriggerCondition<Event>>,
    },

    /// Triggers after all given conditions have been fulfilled in any order.
    ///
    /// Compared to [`Self::Sequence`](TriggerCondition::Sequence), this does not require to fulfil the conditions in any order.
    And {
        /// The conditions to fulfil.
        conditions: Vec<TriggerCondition<Event>>,
    },

    /// Triggers after one of the given conditions have been fulfilled.
    Or {
        /// The conditions to fulfil.
        conditions: Vec<TriggerCondition<Event>>,
    },

    /// Triggers after a given number of the given conditions have been fulfilled.
    AnyN {
        /// The conditions to fulfil.
        conditions: Vec<TriggerCondition<Event>>,
        /// The amount of conditions that need to be fulfilled.
        n: usize,
    },
}

/// A compiled trigger condition.
///
/// This
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CompiledTriggerCondition<Event: TriggerEvent> {
    pub(crate) kind: CompiledTriggerConditionKind<Event>,
    pub(crate) completed: bool,
    pub(crate) required_progress: f64,
    pub(crate) current_progress: f64,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub(crate) enum CompiledTriggerConditionKind<Event: TriggerEvent> {
    None,
    Never,
    EventCount {
        identifier: Event::Identifier,
        count: usize,
        required: usize,
    },
    Geq {
        event: Event,
        fulfilled: bool,
    },
    Sequence {
        current_index: usize,
        conditions: Vec<CompiledTriggerCondition<Event>>,
    },
    And {
        conditions: Vec<CompiledTriggerCondition<Event>>,
        fulfilled_conditions: Vec<CompiledTriggerCondition<Event>>,
    },
    Or {
        conditions: Vec<CompiledTriggerCondition<Event>>,
        fulfilled_conditions: Vec<CompiledTriggerCondition<Event>>,
    },
    AnyN {
        conditions: Vec<CompiledTriggerCondition<Event>>,
        fulfilled_conditions: Vec<CompiledTriggerCondition<Event>>,
        n: usize,
    },
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TriggerConditionUpdate<Identifier> {
    Subscribe(Identifier),
    Unsubscribe(Identifier),
}

impl<Event> TriggerCondition<Event> {
    /// Compile this trigger condition.
    ///
    /// Raw event information is transformed into a more compact identifier for a matching compiled event.
    pub fn compile<EventCompiler: Fn(Event) -> CompiledEvent, CompiledEvent: TriggerEvent>(
        self,
        event_compiler: &EventCompiler,
    ) -> CompiledTriggerCondition<CompiledEvent> {
        CompiledTriggerCondition::new(match self {
            TriggerCondition::None => CompiledTriggerConditionKind::None,
            TriggerCondition::Never => CompiledTriggerConditionKind::Never,
            TriggerCondition::EventCount { event, required } => {
                CompiledTriggerConditionKind::EventCount {
                    identifier: event_compiler(event).identifier(),
                    count: 0,
                    required,
                }
            }
            TriggerCondition::Geq { event } => CompiledTriggerConditionKind::Geq {
                event: event_compiler(event),
                fulfilled: false,
            },
            TriggerCondition::Sequence { conditions } => {
                let conditions = conditions
                    .into_iter()
                    .map(|condition| {
                        let condition = condition.compile(event_compiler);
                        // Sequences are not allowed to contain `None` conditions.
                        assert!(!condition.completed());
                        condition
                    })
                    .collect();
                CompiledTriggerConditionKind::Sequence {
                    current_index: 0,
                    conditions,
                }
            }
            TriggerCondition::And { conditions } => {
                let mut compiled_conditions = Vec::new();
                let mut compiled_fulfilled_conditions = Vec::new();
                for condition in conditions {
                    let compiled_condition = condition.compile(event_compiler);
                    if compiled_condition.completed() {
                        compiled_fulfilled_conditions.push(compiled_condition);
                    } else {
                        compiled_conditions.push(compiled_condition);
                    }
                }
                CompiledTriggerConditionKind::And {
                    conditions: compiled_conditions,
                    fulfilled_conditions: compiled_fulfilled_conditions,
                }
            }
            TriggerCondition::Or { conditions } => {
                let mut compiled_conditions = Vec::new();
                let mut compiled_fulfilled_conditions = Vec::new();
                for condition in conditions {
                    let compiled_condition = condition.compile(event_compiler);
                    if compiled_condition.completed() {
                        compiled_fulfilled_conditions.push(compiled_condition);
                    } else {
                        compiled_conditions.push(compiled_condition);
                    }
                }
                CompiledTriggerConditionKind::Or {
                    conditions: compiled_conditions,
                    fulfilled_conditions: compiled_fulfilled_conditions,
                }
            }
            TriggerCondition::AnyN { conditions, n } => {
                let mut compiled_conditions = Vec::new();
                let mut compiled_fulfilled_conditions = Vec::new();
                for condition in conditions {
                    let compiled_condition = condition.compile(event_compiler);
                    if compiled_condition.completed() {
                        compiled_fulfilled_conditions.push(compiled_condition);
                    } else {
                        compiled_conditions.push(compiled_condition);
                    }
                }
                CompiledTriggerConditionKind::AnyN {
                    conditions: compiled_conditions,
                    fulfilled_conditions: compiled_fulfilled_conditions,
                    n,
                }
            }
        })
    }
}

impl<Event: TriggerEvent> CompiledTriggerCondition<Event> {
    pub(crate) fn new(kind: CompiledTriggerConditionKind<Event>) -> Self {
        Self {
            required_progress: kind.required_progress(),
            current_progress: 0.0,
            completed: kind.completed(),
            kind,
        }
    }

    /// Returns the required progress of the compiled trigger condition.
    pub fn required_progress(&self) -> f64 {
        self.required_progress
    }

    /// Returns the current progress of the compiled trigger condition.
    pub fn current_progress(&self) -> f64 {
        assert!(self.current_progress.is_finite());
        self.current_progress
    }

    /// Returns true if the compiled trigger condition is completed.
    pub fn completed(&self) -> bool {
        self.completed
    }

    pub(crate) fn execute_event(
        &mut self,
        event: &Event,
    ) -> (Vec<TriggerConditionUpdate<Event::Identifier>>, bool, f64) {
        assert!(!self.completed);
        let (trigger_condition_update, result, current_progress) = self.kind.execute_event(event);
        assert!(current_progress >= self.current_progress - 1e-6);
        self.current_progress = current_progress;
        self.completed = result;
        (trigger_condition_update, result, self.current_progress)
    }

    pub(crate) fn subscriptions(&self) -> Vec<Event::Identifier> {
        if self.completed {
            return Default::default();
        }

        match &self.kind {
            CompiledTriggerConditionKind::None => Default::default(),
            CompiledTriggerConditionKind::Never => Default::default(),
            CompiledTriggerConditionKind::EventCount { identifier, .. } => vec![identifier.clone()],
            CompiledTriggerConditionKind::Geq { event, .. } => vec![event.identifier()],
            CompiledTriggerConditionKind::Sequence {
                current_index,
                conditions,
            } => conditions[*current_index].subscriptions(),
            CompiledTriggerConditionKind::And { conditions, .. } => conditions
                .iter()
                .flat_map(|condition| condition.subscriptions())
                .collect(),
            CompiledTriggerConditionKind::Or { conditions, .. } => conditions
                .iter()
                .flat_map(|condition| condition.subscriptions())
                .collect(),
            CompiledTriggerConditionKind::AnyN { conditions, .. } => conditions
                .iter()
                .flat_map(|condition| condition.subscriptions())
                .collect(),
        }
    }
}

impl<Event: TriggerEvent> CompiledTriggerConditionKind<Event> {
    fn required_progress(&self) -> f64 {
        match self {
            CompiledTriggerConditionKind::None => 0.0,
            CompiledTriggerConditionKind::Never => 1.0,
            CompiledTriggerConditionKind::EventCount { required, .. } => *required as f64,
            CompiledTriggerConditionKind::Geq { .. } => 1.0,
            CompiledTriggerConditionKind::Sequence { conditions, .. } => conditions
                .iter()
                .map(|condition| condition.required_progress())
                .sum(),
            CompiledTriggerConditionKind::And {
                conditions,
                fulfilled_conditions,
            } => conditions
                .iter()
                .chain(fulfilled_conditions.iter())
                .map(|condition| condition.required_progress())
                .sum(),
            CompiledTriggerConditionKind::Or {
                conditions,
                fulfilled_conditions,
            } => conditions
                .iter()
                .chain(fulfilled_conditions.iter())
                .map(|condition| condition.required_progress())
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(0.0),
            CompiledTriggerConditionKind::AnyN {
                conditions,
                fulfilled_conditions,
                n,
            } => {
                let mut required_progresses: Vec<_> = conditions
                    .iter()
                    .chain(fulfilled_conditions.iter())
                    .map(|condition| condition.required_progress())
                    .collect();
                required_progresses.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
                required_progresses.iter().take(*n).sum()
            }
        }
    }

    fn completed(&self) -> bool {
        match self {
            CompiledTriggerConditionKind::None => true,
            CompiledTriggerConditionKind::Never => false,
            CompiledTriggerConditionKind::EventCount {
                count, required, ..
            } => count >= required,
            CompiledTriggerConditionKind::Geq { fulfilled, .. } => *fulfilled,
            CompiledTriggerConditionKind::Sequence {
                current_index,
                conditions,
            } => *current_index >= conditions.len(),
            CompiledTriggerConditionKind::And { conditions, .. } => conditions.is_empty(),
            CompiledTriggerConditionKind::Or { conditions, .. } => conditions.is_empty(),
            CompiledTriggerConditionKind::AnyN {
                fulfilled_conditions,
                n,
                ..
            } => fulfilled_conditions.len() >= *n,
        }
    }

    fn execute_event(
        &mut self,
        event: &Event,
    ) -> (Vec<TriggerConditionUpdate<Event::Identifier>>, bool, f64) {
        match self {
            CompiledTriggerConditionKind::None => (Default::default(), true, 0.0),
            CompiledTriggerConditionKind::Never => (Default::default(), false, 0.0),
            CompiledTriggerConditionKind::EventCount {
                identifier: counted_identifier,
                count,
                required,
            } => {
                assert!(count < required);
                let identifier = event.identifier();
                if *counted_identifier == identifier {
                    *count += 1;
                }

                assert!(count <= required);
                if count == required {
                    (
                        vec![TriggerConditionUpdate::Unsubscribe(
                            counted_identifier.clone(),
                        )],
                        true,
                        *count as f64,
                    )
                } else {
                    (Default::default(), count >= required, *count as f64)
                }
            }
            CompiledTriggerConditionKind::Geq {
                event: reference_event,
                fulfilled,
            } => {
                assert!(!*fulfilled);
                if event.value_geq(reference_event).unwrap() {
                    *fulfilled = true;
                    return (
                        vec![TriggerConditionUpdate::Unsubscribe(
                            reference_event.identifier(),
                        )],
                        true,
                        1.0,
                    );
                }
                (
                    vec![],
                    false,
                    event.value_geq_progress(reference_event).unwrap(),
                )
            }
            CompiledTriggerConditionKind::Sequence {
                current_index,
                conditions,
            } => {
                assert!(*current_index < conditions.len());
                let progress_base: f64 = conditions
                    .iter()
                    .take(*current_index)
                    .map(|condition| condition.required_progress())
                    .sum();
                let (mut trigger_condition_update, result, current_progress) =
                    conditions[*current_index].execute_event(event);
                if result {
                    let progress_base =
                        progress_base + conditions[*current_index].required_progress();
                    *current_index += 1;

                    if *current_index < conditions.len() {
                        trigger_condition_update.extend(
                            conditions[*current_index]
                                .subscriptions()
                                .into_iter()
                                .map(TriggerConditionUpdate::Subscribe),
                        );
                        (
                            trigger_condition_update,
                            false,
                            progress_base + conditions[*current_index].current_progress(),
                        )
                    } else {
                        (trigger_condition_update, true, progress_base)
                    }
                } else {
                    (
                        trigger_condition_update,
                        false,
                        progress_base + current_progress,
                    )
                }
            }
            CompiledTriggerConditionKind::And {
                conditions,
                fulfilled_conditions,
            } => {
                assert!(!conditions.is_empty());
                let mut trigger_condition_updates = Vec::new();
                let mut current_progress: f64 = fulfilled_conditions
                    .iter()
                    .map(|condition| condition.required_progress())
                    .sum();

                // TODO replace with drain_filter once stable
                let mut i = 0;
                while i < conditions.len() {
                    let (mut local_trigger_condition_updates, result, progress) =
                        conditions[i].execute_event(event);
                    trigger_condition_updates.append(&mut local_trigger_condition_updates);
                    if result {
                        current_progress += conditions[i].required_progress();
                        fulfilled_conditions.push(conditions.remove(i));
                    } else {
                        current_progress += progress;
                        i += 1;
                    }
                }
                (
                    trigger_condition_updates,
                    conditions.is_empty(),
                    current_progress,
                )
            }
            CompiledTriggerConditionKind::Or {
                conditions,
                fulfilled_conditions,
            } => {
                assert!(fulfilled_conditions.is_empty());
                let mut trigger_condition_updates = Vec::new();
                let mut current_progress: f64 = 0.0;

                // TODO replace with drain_filter once stable
                let mut i = 0;
                while i < conditions.len() {
                    let (mut local_trigger_condition_updates, result, progress) =
                        conditions[i].execute_event(event);
                    trigger_condition_updates.append(&mut local_trigger_condition_updates);
                    if result {
                        current_progress = 1.0;
                        fulfilled_conditions.push(conditions.remove(i));
                    } else {
                        current_progress =
                            current_progress.max(progress / conditions[i].required_progress());
                        i += 1;
                    }
                }

                let result = !fulfilled_conditions.is_empty();
                if result {
                    trigger_condition_updates.extend(conditions.iter().flat_map(|condition| {
                        condition
                            .subscriptions()
                            .into_iter()
                            .map(TriggerConditionUpdate::Unsubscribe)
                    }));
                }

                (
                    trigger_condition_updates,
                    result,
                    current_progress * self.required_progress(),
                )
            }
            CompiledTriggerConditionKind::AnyN {
                conditions,
                fulfilled_conditions,
                n,
            } => {
                assert!(fulfilled_conditions.len() < *n);
                let mut trigger_condition_updates = Vec::new();
                let mut relative_progresses = vec![1.0; fulfilled_conditions.len()];

                // TODO replace with drain_filter once stable
                let mut i = 0;
                while i < conditions.len() {
                    let (mut local_trigger_condition_updates, result, progress) =
                        conditions[i].execute_event(event);
                    trigger_condition_updates.append(&mut local_trigger_condition_updates);
                    if result {
                        relative_progresses.push(1.0);
                        fulfilled_conditions.push(conditions.remove(i));
                    } else {
                        relative_progresses.push(progress / conditions[i].required_progress());
                        i += 1;
                    }
                }

                let result = fulfilled_conditions.len() >= *n;
                if result {
                    trigger_condition_updates.extend(conditions.iter().flat_map(|condition| {
                        condition
                            .subscriptions()
                            .into_iter()
                            .map(TriggerConditionUpdate::Unsubscribe)
                    }));
                }

                relative_progresses.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
                let current_progress = relative_progresses.iter().rev().take(*n).sum::<f64>()
                    / (*n as f64)
                    * self.required_progress();
                (trigger_condition_updates, result, current_progress)
            }
        }
    }
}

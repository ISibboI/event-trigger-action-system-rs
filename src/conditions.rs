use crate::triggers::TriggerEvent;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TriggerCondition<Event: TriggerEvent> {
    pub(crate) kind: TriggerConditionKind<Event>,
    pub(crate) completed: bool,
    pub(crate) required_progress: f64,
    pub(crate) current_progress: f64,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TriggerConditionKind<Event: TriggerEvent> {
    None,
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
        conditions: Vec<TriggerCondition<Event>>,
    },
    And {
        conditions: Vec<TriggerCondition<Event>>,
        fulfilled_conditions: Vec<TriggerCondition<Event>>,
    },
    Or {
        conditions: Vec<TriggerCondition<Event>>,
        fulfilled_conditions: Vec<TriggerCondition<Event>>,
    },
    AnyN {
        conditions: Vec<TriggerCondition<Event>>,
        fulfilled_conditions: Vec<TriggerCondition<Event>>,
        n: usize,
    },
}

#[derive(Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TriggerConditionUpdate<Identifier> {
    Subscribe(Identifier),
    Unsubscribe(Identifier),
}

impl<Event: TriggerEvent> TriggerCondition<Event> {
    pub(crate) fn new(kind: TriggerConditionKind<Event>) -> Self {
        Self {
            required_progress: kind.required_progress(),
            current_progress: 0.0,
            completed: kind.completed(),
            kind,
        }
    }

    pub fn required_progress(&self) -> f64 {
        self.required_progress
    }

    pub fn current_progress(&self) -> f64 {
        assert!(self.current_progress.is_finite());
        self.current_progress
    }

    pub fn completed(&self) -> bool {
        self.completed
    }

    pub(crate) fn execute_event(
        &mut self,
        event: &Event,
    ) -> (Vec<TriggerConditionUpdate<Event::Identifier>>, bool, f64) {
        assert!(!self.completed);
        let (trigger_condition_update, result, current_progress) = self.kind.execute_event(event);
        self.current_progress = current_progress;
        self.completed = result;
        (trigger_condition_update, result, current_progress)
    }

    pub(crate) fn subscriptions(&self) -> Vec<Event::Identifier> {
        if self.completed {
            return Default::default();
        }

        match &self.kind {
            TriggerConditionKind::None => Default::default(),
            TriggerConditionKind::EventCount { identifier, .. } => vec![identifier.clone()],
            TriggerConditionKind::Geq { event, .. } => vec![event.identifier()],
            TriggerConditionKind::Sequence {
                current_index,
                conditions,
            } => conditions[*current_index].subscriptions(),
            TriggerConditionKind::And { conditions, .. } => conditions
                .iter()
                .flat_map(|condition| condition.subscriptions())
                .collect(),
            TriggerConditionKind::Or { conditions, .. } => conditions
                .iter()
                .flat_map(|condition| condition.subscriptions())
                .collect(),
            TriggerConditionKind::AnyN { conditions, .. } => conditions
                .iter()
                .flat_map(|condition| condition.subscriptions())
                .collect(),
        }
    }
}

impl<Event: TriggerEvent> TriggerConditionKind<Event> {
    fn required_progress(&self) -> f64 {
        match self {
            TriggerConditionKind::None => 0.0,
            TriggerConditionKind::EventCount { required, .. } => *required as f64,
            TriggerConditionKind::Geq { .. } => 1.0,
            TriggerConditionKind::Sequence { conditions, .. } => conditions
                .iter()
                .map(|condition| condition.required_progress())
                .sum(),
            TriggerConditionKind::And {
                conditions,
                fulfilled_conditions,
            } => conditions
                .iter()
                .chain(fulfilled_conditions.iter())
                .map(|condition| condition.required_progress())
                .sum(),
            TriggerConditionKind::Or {
                conditions,
                fulfilled_conditions,
            } => conditions
                .iter()
                .chain(fulfilled_conditions.iter())
                .map(|condition| condition.required_progress())
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(0.0),
            TriggerConditionKind::AnyN {
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
            TriggerConditionKind::None => true,
            TriggerConditionKind::EventCount {
                count, required, ..
            } => count >= required,
            TriggerConditionKind::Geq { fulfilled, .. } => *fulfilled,
            TriggerConditionKind::Sequence {
                current_index,
                conditions,
            } => *current_index >= conditions.len(),
            TriggerConditionKind::And { conditions, .. } => conditions.is_empty(),
            TriggerConditionKind::Or { conditions, .. } => conditions.is_empty(),
            TriggerConditionKind::AnyN {
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
            TriggerConditionKind::None => (Default::default(), true, 0.0),
            TriggerConditionKind::EventCount {
                identifier: counted_identifier,
                count,
                required,
            } => {
                let identifier = event.identifier();
                if *counted_identifier == identifier {
                    assert!(count < required);
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
            TriggerConditionKind::Geq {
                event: reference_event,
                fulfilled,
            } => {
                assert!(!*fulfilled);
                if event.value_geq(reference_event).unwrap() {
                    *fulfilled = true;
                    (
                        vec![TriggerConditionUpdate::Unsubscribe(
                            reference_event.identifier(),
                        )],
                        true,
                        1.0,
                    )
                } else {
                    (
                        vec![],
                        false,
                        event.value_geq_progress(reference_event).unwrap(),
                    )
                }
            }
            TriggerConditionKind::Sequence {
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
            TriggerConditionKind::And {
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
            TriggerConditionKind::Or {
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
            TriggerConditionKind::AnyN {
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

use crate::TriggerCondition;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};

/// Constructs a trigger condition that triggers immediately.
pub fn none<Event>() -> TriggerCondition<Event> {
    TriggerCondition::None
}

/// Constructs a trigger condition that never triggers.
pub fn never<Event>() -> TriggerCondition<Event> {
    TriggerCondition::Never
}

/// Constructs a trigger condition that triggers after the given event has been received the given amount of times.
pub fn event_count<Event>(event: Event, required: usize) -> TriggerCondition<Event> {
    TriggerCondition::EventCount { event, required }
}

/// Constructs a trigger condition that triggers after an event has been received that is greater than the reference event.
pub fn gt<Event>(reference_event: Event) -> TriggerCondition<Event> {
    TriggerCondition::Greater { reference_event }
}

/// Constructs a trigger condition that triggers after an event has been received that is greater than or equal to the reference event.
pub fn geq<Event>(reference_event: Event) -> TriggerCondition<Event> {
    TriggerCondition::GreaterOrEqual { reference_event }
}

/// Constructs a trigger condition that triggers after an event has been received that is equal to the reference event.
pub fn eq<Event>(reference_event: Event) -> TriggerCondition<Event> {
    TriggerCondition::Equal { reference_event }
}

/// Constructs a trigger condition that triggers after an event has been received that is less than or equal to the reference event.
pub fn leq<Event>(reference_event: Event) -> TriggerCondition<Event> {
    TriggerCondition::LessOrEqual { reference_event }
}

/// Constructs a trigger condition that triggers after an event has been received that is less than the reference event.
pub fn lt<Event>(reference_event: Event) -> TriggerCondition<Event> {
    TriggerCondition::Less { reference_event }
}

/// Constructs a trigger condition that triggers after all given conditions have triggered.
pub fn and<Event>(conditions: Vec<TriggerCondition<Event>>) -> TriggerCondition<Event> {
    TriggerCondition::And { conditions }
}

/// Constructs a trigger condition that triggers after any of the given conditions have triggered.
pub fn or<Event>(conditions: Vec<TriggerCondition<Event>>) -> TriggerCondition<Event> {
    TriggerCondition::Or { conditions }
}

/// Constructs a trigger condition that triggers after all given conditions have triggered in sequence.
///
/// This works by having a pointer to the current active condition in the sequence.
/// Whenever a condition is fulfilled, the pointer gets moved to the right.
/// If it has visited all conditions, then this condition triggers.
pub fn sequence<Event>(conditions: Vec<TriggerCondition<Event>>) -> TriggerCondition<Event> {
    TriggerCondition::Sequence { conditions }
}

/// Constructs a trigger condition that triggers after the given amount of given trigger conditions have triggered.
pub fn any_n<Event>(conditions: Vec<TriggerCondition<Event>>, n: usize) -> TriggerCondition<Event> {
    TriggerCondition::AnyN { conditions, n }
}

impl<Event: Clone> BitAndAssign for TriggerCondition<Event> {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = self.clone() & rhs;
    }
}

impl<Event: Clone> BitOrAssign for TriggerCondition<Event> {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = self.clone() | rhs;
    }
}

impl<Event> BitAnd for TriggerCondition<Event> {
    type Output = TriggerCondition<Event>;

    fn bitand(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (
                TriggerCondition::And {
                    conditions: mut conditions_self,
                },
                TriggerCondition::And {
                    conditions: mut conditions_rhs,
                },
            ) => {
                conditions_self.append(&mut conditions_rhs);
                TriggerCondition::And {
                    conditions: conditions_self,
                }
            }
            (
                TriggerCondition::And {
                    conditions: mut conditions_self,
                },
                rhs,
            ) => {
                conditions_self.push(rhs);
                TriggerCondition::And {
                    conditions: conditions_self,
                }
            }
            (
                lhs,
                TriggerCondition::And {
                    conditions: mut conditions_rhs,
                },
            ) => {
                conditions_rhs.push(lhs);
                TriggerCondition::And {
                    conditions: conditions_rhs,
                }
            }
            (lhs, rhs) => {
                let conditions = vec![lhs, rhs];
                TriggerCondition::And { conditions }
            }
        }
    }
}

impl<Event> BitOr for TriggerCondition<Event> {
    type Output = TriggerCondition<Event>;

    fn bitor(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (
                TriggerCondition::Or {
                    conditions: mut conditions_self,
                },
                TriggerCondition::Or {
                    conditions: mut conditions_rhs,
                },
            ) => {
                conditions_self.append(&mut conditions_rhs);
                TriggerCondition::Or {
                    conditions: conditions_self,
                }
            }
            (
                TriggerCondition::Or {
                    conditions: mut conditions_self,
                },
                rhs,
            ) => {
                conditions_self.push(rhs);
                TriggerCondition::Or {
                    conditions: conditions_self,
                }
            }
            (
                lhs,
                TriggerCondition::Or {
                    conditions: mut conditions_rhs,
                },
            ) => {
                conditions_rhs.push(lhs);
                TriggerCondition::Or {
                    conditions: conditions_rhs,
                }
            }
            (lhs, rhs) => {
                let conditions = vec![lhs, rhs];
                TriggerCondition::Or { conditions }
            }
        }
    }
}

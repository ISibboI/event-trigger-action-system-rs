use crate::{TriggerCondition, TriggerEvent};
use std::ops::{BitAnd, BitOr};

pub fn none<Event>() -> TriggerCondition<Event> {
    TriggerCondition::None
}

pub fn never<Event>() -> TriggerCondition<Event> {
    TriggerCondition::Never
}

pub fn event_count<Event>(event: Event, required: usize) -> TriggerCondition<Event> {
    TriggerCondition::EventCount { event, required }
}

pub fn geq<Event>(event: Event) -> TriggerCondition<Event> {
    TriggerCondition::Geq { event }
}

pub fn sequence<Event>(conditions: Vec<TriggerCondition<Event>>) -> TriggerCondition<Event> {
    TriggerCondition::Sequence { conditions }
}

pub fn any_n<Event>(conditions: Vec<TriggerCondition<Event>>, n: usize) -> TriggerCondition<Event> {
    TriggerCondition::AnyN { conditions, n }
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

impl<Event: TriggerEvent> BitOr for TriggerCondition<Event> {
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

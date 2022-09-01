use crate::conditions::TriggerConditionKind;
use crate::TriggerCondition;
use std::ops::{BitAnd, BitOr};

pub fn none<Event>() -> TriggerCondition<Event> {
    TriggerCondition::new(TriggerConditionKind::None)
}

pub fn event_count<Event>(event: Event, required: usize) -> TriggerCondition<Event> {
    TriggerCondition::new(TriggerConditionKind::EventCount {
        event,
        required,
        count: 0,
    })
}

pub fn sequence<Event>(conditions: Vec<TriggerCondition<Event>>) -> TriggerCondition<Event> {
    TriggerCondition::new(TriggerConditionKind::Sequence {
        conditions,
        current_index: 0,
    })
}

pub fn any_n<Event>(conditions: Vec<TriggerCondition<Event>>, n: usize) -> TriggerCondition<Event> {
    if n == 0 {
        none()
    } else {
        TriggerCondition::new(TriggerConditionKind::AnyN {
            conditions,
            fulfilled_conditions: Default::default(),
            n,
        })
    }
}

impl<Event> BitAnd for TriggerCondition<Event> {
    type Output = TriggerCondition<Event>;

    fn bitand(self, rhs: Self) -> Self::Output {
        let completed = self.completed && rhs.completed;
        let required_progress = self.required_progress + rhs.required_progress;
        let current_progress = self.current_progress + rhs.current_progress;
        match (self.kind, rhs.kind) {
            (
                TriggerConditionKind::And {
                    conditions: mut conditions_self,
                    fulfilled_conditions: mut fulfilled_conditions_self,
                },
                TriggerConditionKind::And {
                    conditions: mut conditions_rhs,
                    fulfilled_conditions: mut fulfilled_conditions_rhs,
                },
            ) => {
                conditions_self.append(&mut conditions_rhs);
                fulfilled_conditions_self.append(&mut fulfilled_conditions_rhs);
                TriggerCondition {
                    kind: TriggerConditionKind::And {
                        conditions: conditions_self,
                        fulfilled_conditions: fulfilled_conditions_self,
                    },
                    completed,
                    required_progress,
                    current_progress,
                }
            }
            (
                TriggerConditionKind::And {
                    conditions: mut conditions_self,
                    fulfilled_conditions: mut fulfilled_conditions_self,
                },
                rhs_kind,
            ) => {
                let rhs = Self {
                    kind: rhs_kind,
                    completed: rhs.completed,
                    required_progress: rhs.required_progress,
                    current_progress: rhs.current_progress,
                };
                if rhs.completed {
                    fulfilled_conditions_self.push(rhs);
                } else {
                    conditions_self.push(rhs);
                }
                TriggerCondition {
                    kind: TriggerConditionKind::And {
                        conditions: conditions_self,
                        fulfilled_conditions: fulfilled_conditions_self,
                    },
                    completed,
                    required_progress,
                    current_progress,
                }
            }
            (
                lhs_kind,
                TriggerConditionKind::And {
                    conditions: mut conditions_rhs,
                    fulfilled_conditions: mut fulfilled_conditions_rhs,
                },
            ) => {
                let lhs = Self {
                    kind: lhs_kind,
                    completed: self.completed,
                    required_progress: self.required_progress,
                    current_progress: self.current_progress,
                };
                if lhs.completed {
                    fulfilled_conditions_rhs.push(lhs);
                } else {
                    conditions_rhs.push(lhs);
                }
                TriggerCondition {
                    kind: TriggerConditionKind::And {
                        conditions: conditions_rhs,
                        fulfilled_conditions: fulfilled_conditions_rhs,
                    },
                    completed,
                    required_progress,
                    current_progress,
                }
            }
            (lhs_kind, rhs_kind) => {
                let mut conditions = Vec::new();
                let mut fulfilled_conditions = Vec::new();
                let lhs = Self {
                    kind: lhs_kind,
                    completed: self.completed,
                    required_progress: self.required_progress,
                    current_progress: self.current_progress,
                };
                let rhs = Self {
                    kind: rhs_kind,
                    completed: rhs.completed,
                    required_progress: rhs.required_progress,
                    current_progress: rhs.current_progress,
                };
                if self.completed {
                    fulfilled_conditions.push(lhs);
                } else {
                    conditions.push(lhs);
                }
                if rhs.completed {
                    fulfilled_conditions.push(rhs);
                } else {
                    conditions.push(rhs);
                }
                TriggerCondition {
                    kind: TriggerConditionKind::And {
                        conditions,
                        fulfilled_conditions,
                    },
                    completed,
                    required_progress,
                    current_progress,
                }
            }
        }
    }
}

impl<Event> BitOr for TriggerCondition<Event> {
    type Output = TriggerCondition<Event>;

    fn bitor(self, rhs: Self) -> Self::Output {
        let completed = self.completed || rhs.completed;
        let required_progress = self.required_progress.min(rhs.required_progress);
        let current_progress = (if self.required_progress.abs() == 0.0 {
            0.0
        } else {
            self.current_progress / self.required_progress
        })
        .max(if rhs.required_progress.abs() == 0.0 {
            0.0
        } else {
            rhs.current_progress / rhs.required_progress
        }) * required_progress;
        match (self.kind, rhs.kind) {
            (
                TriggerConditionKind::Or {
                    conditions: mut conditions_self,
                    fulfilled_conditions: mut fulfilled_conditions_self,
                },
                TriggerConditionKind::Or {
                    conditions: mut conditions_rhs,
                    fulfilled_conditions: mut fulfilled_conditions_rhs,
                },
            ) => {
                conditions_self.append(&mut conditions_rhs);
                fulfilled_conditions_self.append(&mut fulfilled_conditions_rhs);
                TriggerCondition {
                    kind: TriggerConditionKind::Or {
                        conditions: conditions_self,
                        fulfilled_conditions: fulfilled_conditions_self,
                    },
                    completed,
                    required_progress,
                    current_progress,
                }
            }
            (
                TriggerConditionKind::Or {
                    conditions: mut conditions_self,
                    fulfilled_conditions: mut fulfilled_conditions_self,
                },
                rhs_kind,
            ) => {
                let rhs = Self {
                    kind: rhs_kind,
                    completed: rhs.completed,
                    required_progress: rhs.required_progress,
                    current_progress: rhs.current_progress,
                };
                if rhs.completed {
                    fulfilled_conditions_self.push(rhs);
                } else {
                    conditions_self.push(rhs);
                }
                TriggerCondition {
                    kind: TriggerConditionKind::Or {
                        conditions: conditions_self,
                        fulfilled_conditions: fulfilled_conditions_self,
                    },
                    completed,
                    required_progress,
                    current_progress,
                }
            }
            (
                lhs_kind,
                TriggerConditionKind::Or {
                    conditions: mut conditions_rhs,
                    fulfilled_conditions: mut fulfilled_conditions_rhs,
                },
            ) => {
                let lhs = Self {
                    kind: lhs_kind,
                    completed: self.completed,
                    required_progress: self.required_progress,
                    current_progress: self.current_progress,
                };
                if lhs.completed {
                    fulfilled_conditions_rhs.push(lhs);
                } else {
                    conditions_rhs.push(lhs);
                }
                TriggerCondition {
                    kind: TriggerConditionKind::Or {
                        conditions: conditions_rhs,
                        fulfilled_conditions: fulfilled_conditions_rhs,
                    },
                    completed,
                    required_progress,
                    current_progress,
                }
            }
            (lhs_kind, rhs_kind) => {
                let mut conditions = Vec::new();
                let mut fulfilled_conditions = Vec::new();
                let lhs = Self {
                    kind: lhs_kind,
                    completed: self.completed,
                    required_progress: self.required_progress,
                    current_progress: self.current_progress,
                };
                let rhs = Self {
                    kind: rhs_kind,
                    completed: rhs.completed,
                    required_progress: rhs.required_progress,
                    current_progress: rhs.current_progress,
                };
                if self.completed {
                    fulfilled_conditions.push(lhs);
                } else {
                    conditions.push(lhs);
                }
                if rhs.completed {
                    fulfilled_conditions.push(rhs);
                } else {
                    conditions.push(rhs);
                }
                TriggerCondition {
                    kind: TriggerConditionKind::Or {
                        conditions,
                        fulfilled_conditions,
                    },
                    completed,
                    required_progress,
                    current_progress,
                }
            }
        }
    }
}

use event_trigger_action_system::{
    event_count, geq, none, sequence, Trigger, TriggerAction, TriggerConditionUpdate, TriggerEvent,
    TriggerIdentifier, Triggers,
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
enum GameAction {
    ActivateQuest { id: QuestHandle },
    CompleteQuest { id: QuestHandle },
    FailQuest { id: QuestHandle },
    ActivateMonster { id: MonsterHandle },
    DeactivateMonster { id: MonsterHandle },
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
enum GameEvent {
    Action(GameAction),
    KilledMonster { id: MonsterHandle },
    FailedMonster { id: MonsterHandle },
    HealthChanged { health: usize },
    MonsterHealthChanged { id: MonsterHandle, health: usize },
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
enum GameEventIdentifier {
    Action(GameAction),
    KilledMonster { id: MonsterHandle },
    FailedMonster { id: MonsterHandle },
    HealthChanged,
    MonsterHealthChanged { id: MonsterHandle },
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
struct QuestHandle(usize);
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
struct MonsterHandle(usize);

impl TriggerAction for GameAction {}

impl TriggerIdentifier for GameEventIdentifier {}

impl TriggerEvent for GameEvent {
    type Action = GameAction;
    type Identifier = GameEventIdentifier;

    fn identifier(&self) -> Self::Identifier {
        match self {
            GameEvent::Action(action) => GameEventIdentifier::Action(action.clone()),
            GameEvent::KilledMonster { id } => GameEventIdentifier::KilledMonster { id: *id },
            GameEvent::FailedMonster { id } => GameEventIdentifier::FailedMonster { id: *id },
            GameEvent::HealthChanged { .. } => GameEventIdentifier::HealthChanged,
            GameEvent::MonsterHealthChanged { id, .. } => {
                GameEventIdentifier::MonsterHealthChanged { id: *id }
            }
        }
    }

    fn value_geq(&self, other: &Self) -> Option<bool> {
        match (self, other) {
            (
                GameEvent::HealthChanged {
                    health: health_self,
                },
                GameEvent::HealthChanged {
                    health: health_other,
                },
            )
            | (
                GameEvent::MonsterHealthChanged {
                    health: health_self,
                    ..
                },
                GameEvent::MonsterHealthChanged {
                    health: health_other,
                    ..
                },
            ) => Some(health_self >= health_other),
            _ => None,
        }
    }

    fn value_geq_progress(&self, other: &Self) -> Option<f64> {
        match (self, other) {
            (
                GameEvent::HealthChanged {
                    health: health_self,
                },
                GameEvent::HealthChanged {
                    health: health_other,
                },
            )
            | (
                GameEvent::MonsterHealthChanged {
                    health: health_self,
                    ..
                },
                GameEvent::MonsterHealthChanged {
                    health: health_other,
                    ..
                },
            ) => Some((*health_self as f64 / *health_other as f64).clamp(0.0, 1.0)),
            _ => None,
        }
    }
}

impl From<GameAction> for GameEvent {
    fn from(action: GameAction) -> Self {
        Self::Action(action)
    }
}

#[test]
fn test_none() {
    let trigger = Trigger::<GameEvent, GameAction>::new(none(), vec![]).compile(&|x| x, &|x| x);
    assert_eq!(trigger.subscriptions(), vec![]);
    assert_eq!(trigger.progress(), (0.0, 0.0));
}

#[test]
#[should_panic]
fn test_none_panic() {
    let mut trigger = Trigger::<GameEvent, GameAction>::new(none(), vec![]).compile(&|x| x, &|x| x);
    trigger.execute_event(&GameEvent::KilledMonster {
        id: MonsterHandle(0),
    });
}

#[test]
fn test_repeated_action() {
    let mut trigger = Trigger::new(
        event_count(
            GameEvent::KilledMonster {
                id: MonsterHandle(0),
            },
            2,
        ),
        vec![GameAction::CompleteQuest { id: QuestHandle(0) }],
    )
    .compile(&|x| x, &|x| x);
    assert_eq!(
        trigger.subscriptions(),
        vec![GameEventIdentifier::KilledMonster {
            id: MonsterHandle(0)
        }]
    );
    assert_eq!(trigger.progress(), (0.0, 2.0));
    assert!(!trigger.condition().completed());

    assert_eq!(
        trigger.execute_event(&GameEvent::FailedMonster {
            id: MonsterHandle(0)
        }),
        (vec![], vec![])
    );
    assert_eq!(trigger.progress(), (0.0, 2.0));
    assert!(!trigger.condition().completed());

    assert_eq!(
        trigger.execute_event(&GameEvent::KilledMonster {
            id: MonsterHandle(1)
        }),
        (vec![], vec![])
    );
    assert_eq!(trigger.progress(), (0.0, 2.0));
    assert!(!trigger.condition().completed());

    assert_eq!(
        trigger.execute_event(&GameEvent::KilledMonster {
            id: MonsterHandle(0)
        }),
        (vec![], vec![])
    );
    assert_eq!(trigger.progress(), (1.0, 2.0));
    assert!(!trigger.condition().completed());

    assert_eq!(
        trigger.execute_event(&GameEvent::KilledMonster {
            id: MonsterHandle(0)
        }),
        (
            vec![GameAction::CompleteQuest { id: QuestHandle(0) }],
            vec![TriggerConditionUpdate::Unsubscribe(
                GameEventIdentifier::KilledMonster {
                    id: MonsterHandle(0)
                }
            )]
        )
    );
    assert_eq!(trigger.progress(), (2.0, 2.0));
    assert!(trigger.condition().completed());
}

#[test]
fn test_composed_none() {
    let trigger =
        Trigger::<(), ()>::new(none() & none() | none() & none() | none() & none(), vec![])
            .compile(&|x| x, &|x| x);
    dbg!(&trigger);
    assert!(trigger.condition().completed());
    assert_eq!(trigger.progress(), (0.0, 0.0));
}

#[test]
#[should_panic]
fn test_composed_none_panic() {
    let mut trigger =
        Trigger::<(), ()>::new(none() & none() | none() & none() | none() & none(), vec![])
            .compile(&|x| x, &|x| x);
    trigger.execute_event(&());
}

#[test]
fn test_complex() {
    let mut triggers = Triggers::new(vec![
        Trigger::new(
            none(),
            vec![GameAction::ActivateQuest { id: QuestHandle(0) }],
        ),
        Trigger::new(
            event_count(
                GameEvent::KilledMonster {
                    id: MonsterHandle(0),
                },
                2,
            ),
            vec![GameAction::CompleteQuest { id: QuestHandle(0) }],
        ),
        Trigger::new(
            event_count(
                GameEvent::KilledMonster {
                    id: MonsterHandle(0),
                },
                1,
            ),
            vec![GameAction::ActivateQuest { id: QuestHandle(1) }],
        ),
        Trigger::new(
            event_count(
                GameEvent::Action(GameAction::ActivateQuest { id: QuestHandle(1) }),
                1,
            ),
            vec![GameAction::FailQuest { id: QuestHandle(2) }],
        ),
        Trigger::new(
            none(),
            vec![GameAction::ActivateMonster {
                id: MonsterHandle(0),
            }],
        ),
        Trigger::new(
            sequence(vec![
                event_count(
                    GameEvent::FailedMonster {
                        id: MonsterHandle(3),
                    },
                    1,
                ),
                event_count(
                    GameEvent::KilledMonster {
                        id: MonsterHandle(3),
                    },
                    1,
                ),
            ]),
            vec![GameAction::DeactivateMonster {
                id: MonsterHandle(3),
            }],
        ),
    ])
    .compile(&|x| x, &|x| x);
    assert_eq!(
        triggers.consume_action(),
        Some(GameAction::ActivateQuest { id: QuestHandle(0) })
    );
    assert_eq!(
        triggers.consume_action(),
        Some(GameAction::ActivateMonster {
            id: MonsterHandle(0)
        })
    );
    assert_eq!(triggers.consume_action(), None);
    triggers.execute_event(&GameEvent::FailedMonster {
        id: MonsterHandle(2),
    });
    assert_eq!(triggers.consume_action(), None);
    triggers.execute_event(&GameEvent::KilledMonster {
        id: MonsterHandle(0),
    });
    assert_eq!(
        triggers.consume_action(),
        Some(GameAction::ActivateQuest { id: QuestHandle(1) })
    );
    assert_eq!(
        triggers.consume_action(),
        Some(GameAction::FailQuest { id: QuestHandle(2) })
    );
    assert_eq!(triggers.consume_action(), None);
    triggers.execute_event(&GameEvent::KilledMonster {
        id: MonsterHandle(0),
    });
    assert_eq!(
        triggers.consume_action(),
        Some(GameAction::CompleteQuest { id: QuestHandle(0) })
    );
    assert_eq!(triggers.consume_action(), None);
    triggers.execute_event(&GameEvent::KilledMonster {
        id: MonsterHandle(3),
    });
    assert_eq!(triggers.consume_action(), None);
    triggers.execute_event(&GameEvent::FailedMonster {
        id: MonsterHandle(3),
    });
    assert_eq!(triggers.consume_action(), None);
    triggers.execute_event(&GameEvent::KilledMonster {
        id: MonsterHandle(3),
    });
    assert_eq!(
        triggers.consume_action(),
        Some(GameAction::DeactivateMonster {
            id: MonsterHandle(3)
        })
    );
    assert_eq!(triggers.consume_action(), None);
}

#[test]
fn test_geq() {
    let mut triggers = Triggers::new(vec![
        Trigger::new(
            geq(GameEvent::HealthChanged { health: 10 }),
            vec![GameAction::ActivateMonster {
                id: MonsterHandle(0),
            }],
        ),
        Trigger::new(
            sequence(vec![
                event_count(
                    GameEvent::Action(GameAction::ActivateMonster {
                        id: MonsterHandle(0),
                    }),
                    1,
                ),
                geq(GameEvent::MonsterHealthChanged {
                    id: MonsterHandle(0),
                    health: 20,
                }),
            ]),
            vec![GameAction::DeactivateMonster {
                id: MonsterHandle(0),
            }],
        ),
    ])
    .compile(&|x| x, &|x| x);
    assert_eq!(triggers.consume_action(), None);

    triggers.execute_event(&GameEvent::KilledMonster {
        id: MonsterHandle(0),
    });
    assert_eq!(triggers.consume_action(), None);
    triggers.execute_event(&GameEvent::HealthChanged { health: 5 });
    assert_eq!(triggers.consume_action(), None);
    triggers.execute_event(&GameEvent::HealthChanged { health: 10 });
    assert_eq!(
        triggers.consume_action(),
        Some(GameAction::ActivateMonster {
            id: MonsterHandle(0)
        })
    );
    assert_eq!(triggers.consume_action(), None);
    triggers.execute_event(&GameEvent::MonsterHealthChanged {
        id: MonsterHandle(0),
        health: 15,
    });
    assert_eq!(triggers.consume_action(), None);
    triggers.execute_event(&GameEvent::MonsterHealthChanged {
        id: MonsterHandle(1),
        health: 30,
    });
    assert_eq!(triggers.consume_action(), None);
    triggers.execute_event(&GameEvent::MonsterHealthChanged {
        id: MonsterHandle(0),
        health: 23,
    });
    assert_eq!(
        triggers.consume_action(),
        Some(GameAction::DeactivateMonster {
            id: MonsterHandle(0)
        })
    );
    assert_eq!(triggers.consume_action(), None);
}

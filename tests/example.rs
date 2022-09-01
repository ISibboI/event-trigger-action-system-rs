use event_trigger_action_system::{
    event_count, none, sequence, Trigger, TriggerAction, TriggerConditionUpdate, TriggerEvent,
    Triggers,
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
enum GameAction {
    ActivateQuest { id: QuestIdentifier },
    CompleteQuest { id: QuestIdentifier },
    FailQuest { id: QuestIdentifier },
    ActivateMonster { id: MonsterIdentifier },
    DeactivateMonster { id: MonsterIdentifier },
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
enum GameEvent {
    Action(GameAction),
    KilledMonster { id: MonsterIdentifier },
    FailedMonster { id: MonsterIdentifier },
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
struct QuestIdentifier(usize);
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
struct MonsterIdentifier(usize);

impl TriggerAction for GameAction {}

impl TriggerEvent for GameEvent {
    type Action = GameAction;
}

impl From<GameAction> for GameEvent {
    fn from(action: GameAction) -> Self {
        Self::Action(action)
    }
}

#[test]
fn test_none() {
    let trigger = Trigger::<GameEvent>::new(none(), vec![]);
    assert_eq!(trigger.subscriptions(), vec![]);
    assert_eq!(trigger.progress(), (0.0, 0.0));
}

#[test]
#[should_panic]
fn test_none_panic() {
    let mut trigger = Trigger::<GameEvent>::new(none(), vec![]);
    trigger.execute_event(&GameEvent::KilledMonster {
        id: MonsterIdentifier(0),
    });
}

#[test]
fn test_repeated_action() {
    let mut trigger = Trigger::new(
        event_count(
            GameEvent::KilledMonster {
                id: MonsterIdentifier(0),
            },
            2,
        ),
        vec![GameAction::CompleteQuest {
            id: QuestIdentifier(0),
        }],
    );
    assert_eq!(
        trigger.subscriptions(),
        vec![GameEvent::KilledMonster {
            id: MonsterIdentifier(0)
        }]
    );
    assert_eq!(trigger.progress(), (0.0, 2.0));
    assert!(!trigger.condition().completed());

    assert_eq!(
        trigger.execute_event(&GameEvent::FailedMonster {
            id: MonsterIdentifier(0)
        }),
        (vec![], vec![])
    );
    assert_eq!(trigger.progress(), (0.0, 2.0));
    assert!(!trigger.condition().completed());

    assert_eq!(
        trigger.execute_event(&GameEvent::KilledMonster {
            id: MonsterIdentifier(1)
        }),
        (vec![], vec![])
    );
    assert_eq!(trigger.progress(), (0.0, 2.0));
    assert!(!trigger.condition().completed());

    assert_eq!(
        trigger.execute_event(&GameEvent::KilledMonster {
            id: MonsterIdentifier(0)
        }),
        (vec![], vec![])
    );
    assert_eq!(trigger.progress(), (1.0, 2.0));
    assert!(!trigger.condition().completed());

    assert_eq!(
        trigger.execute_event(&GameEvent::KilledMonster {
            id: MonsterIdentifier(0)
        }),
        (
            vec![GameAction::CompleteQuest {
                id: QuestIdentifier(0)
            }],
            vec![TriggerConditionUpdate::Unsubscribe(
                GameEvent::KilledMonster {
                    id: MonsterIdentifier(0)
                }
            )]
        )
    );
    assert_eq!(trigger.progress(), (2.0, 2.0));
    assert!(trigger.condition().completed());
}

#[test]
fn test_composed_none() {
    let trigger = Trigger::<()>::new(none() & none() | none() & none() | none() & none(), vec![]);
    assert!(trigger.condition().completed());
    assert_eq!(trigger.progress(), (0.0, 0.0));
}

#[test]
#[should_panic]
fn test_composed_none_panic() {
    let mut trigger =
        Trigger::<()>::new(none() & none() | none() & none() | none() & none(), vec![]);
    trigger.execute_event(&());
}

#[test]
fn test_complex() {
    let mut triggers = Triggers::new(vec![
        Trigger::new(
            none(),
            vec![GameAction::ActivateQuest {
                id: QuestIdentifier(0),
            }],
        ),
        Trigger::new(
            event_count(
                GameEvent::KilledMonster {
                    id: MonsterIdentifier(0),
                },
                2,
            ),
            vec![GameAction::CompleteQuest {
                id: QuestIdentifier(0),
            }],
        ),
        Trigger::new(
            event_count(
                GameEvent::KilledMonster {
                    id: MonsterIdentifier(0),
                },
                1,
            ),
            vec![GameAction::ActivateQuest {
                id: QuestIdentifier(1),
            }],
        ),
        Trigger::new(
            event_count(
                GameEvent::Action(GameAction::ActivateQuest {
                    id: QuestIdentifier(1),
                }),
                1,
            ),
            vec![GameAction::FailQuest {
                id: QuestIdentifier(2),
            }],
        ),
        Trigger::new(
            none(),
            vec![GameAction::ActivateMonster {
                id: MonsterIdentifier(0),
            }],
        ),
        Trigger::new(
            sequence(vec![
                event_count(
                    GameEvent::FailedMonster {
                        id: MonsterIdentifier(3),
                    },
                    1,
                ),
                event_count(
                    GameEvent::KilledMonster {
                        id: MonsterIdentifier(3),
                    },
                    1,
                ),
            ]),
            vec![GameAction::DeactivateMonster {
                id: MonsterIdentifier(3),
            }],
        ),
    ]);
    assert_eq!(
        triggers.consume_action(),
        Some(GameAction::ActivateQuest {
            id: QuestIdentifier(0)
        })
    );
    assert_eq!(
        triggers.consume_action(),
        Some(GameAction::ActivateMonster {
            id: MonsterIdentifier(0)
        })
    );
    assert_eq!(triggers.consume_action(), None);
    triggers.execute_event(&GameEvent::FailedMonster {
        id: MonsterIdentifier(2),
    });
    assert_eq!(triggers.consume_action(), None);
    triggers.execute_event(&GameEvent::KilledMonster {
        id: MonsterIdentifier(0),
    });
    assert_eq!(
        triggers.consume_action(),
        Some(GameAction::ActivateQuest {
            id: QuestIdentifier(1)
        })
    );
    assert_eq!(
        triggers.consume_action(),
        Some(GameAction::FailQuest {
            id: QuestIdentifier(2)
        })
    );
    assert_eq!(triggers.consume_action(), None);
    triggers.execute_event(&GameEvent::KilledMonster {
        id: MonsterIdentifier(0),
    });
    assert_eq!(
        triggers.consume_action(),
        Some(GameAction::CompleteQuest {
            id: QuestIdentifier(0)
        })
    );
    assert_eq!(triggers.consume_action(), None);
    triggers.execute_event(&GameEvent::KilledMonster {
        id: MonsterIdentifier(3),
    });
    assert_eq!(triggers.consume_action(), None);
    triggers.execute_event(&GameEvent::FailedMonster {
        id: MonsterIdentifier(3),
    });
    assert_eq!(triggers.consume_action(), None);
    triggers.execute_event(&GameEvent::KilledMonster {
        id: MonsterIdentifier(3),
    });
    assert_eq!(
        triggers.consume_action(),
        Some(GameAction::DeactivateMonster {
            id: MonsterIdentifier(3)
        })
    );
    assert_eq!(triggers.consume_action(), None);
}

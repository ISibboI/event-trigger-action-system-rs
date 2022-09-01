use typed_event_triggers::{event_count, none, Trigger, TriggerAction, TriggerConditionUpdate, TriggerEvent};

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
enum GameAction {
    ActivateQuest {id: QuestIdentifier},
    CompleteQuest {id: QuestIdentifier},
    FailQuest {id: QuestIdentifier},
    ActivateMonster {id: MonsterIdentifier},
    DeactivateMonster {id: MonsterIdentifier},
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
enum GameActionIdentifier {
    ActivateQuest,
    CompleteQuest,
    FailQuest,
    ActivateMonster,
    DeactivateMonster,
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
enum GameEvent {
    Action(GameAction),
    KilledMonster {id: MonsterIdentifier},
    FailedMonster {id: MonsterIdentifier},
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
enum GameEventIdentifier {
    Action(GameActionIdentifier),
    KilledMonster,
    FailedMonster,
}

impl TriggerAction for GameAction {
}

impl TriggerEvent for GameEvent {
    type Action = GameAction;
}

impl From<GameAction> for GameEvent {
    fn from(action: GameAction) -> Self {
        Self::Action(action)
    }
}

impl From<GameActionIdentifier> for GameEventIdentifier {
    fn from(action: GameActionIdentifier) -> Self {
        Self::Action(action)
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
struct QuestIdentifier(usize);
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
struct MonsterIdentifier(usize);

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
    trigger.execute_event(&GameEvent::KilledMonster { id: MonsterIdentifier(0) });
}

#[test]
fn test_repeated_action() {
    let mut trigger = Trigger::new(event_count(GameEvent::KilledMonster {id: MonsterIdentifier(0)}, 2), vec![GameAction::CompleteQuest {id: QuestIdentifier(0)}]);
    assert_eq!(trigger.subscriptions(), vec![GameEvent::KilledMonster {id: MonsterIdentifier(0)}]);
    assert_eq!(trigger.progress(), (0.0, 2.0));

    assert_eq!(trigger.execute_event(&GameEvent::FailedMonster {id: MonsterIdentifier(0)}), (vec![], vec![]));
    assert_eq!(trigger.progress(), (0.0, 2.0));

    assert_eq!(trigger.execute_event(&GameEvent::KilledMonster {id: MonsterIdentifier(1)}), (vec![], vec![]));
    assert_eq!(trigger.progress(), (0.0, 2.0));

    assert_eq!(trigger.execute_event(&GameEvent::KilledMonster {id: MonsterIdentifier(0)}), (vec![], vec![]));
    assert_eq!(trigger.progress(), (1.0, 2.0));

    assert_eq!(trigger.execute_event(&GameEvent::KilledMonster {id: MonsterIdentifier(0)}), (vec![GameAction::CompleteQuest {id: QuestIdentifier(0)}], vec![TriggerConditionUpdate::Unsubscribe(GameEvent::KilledMonster {id: MonsterIdentifier(0)})]));
    assert_eq!(trigger.progress(), (2.0, 2.0));
}
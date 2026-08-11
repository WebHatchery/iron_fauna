use super::*;
use crate::model::inventory::GraftCondition;
use crate::state::GameSession;

fn setup() -> (GameData, GameSession) {
    let data = GameData::load().unwrap();
    (data.clone(), GameSession::new_game(&data))
}

fn duelist<'a>(data: &'a GameData, settlement: &str, id: &str) -> &'a DuelistDef {
    data.settlements
        .get(settlement)
        .unwrap()
        .duelist(id)
        .unwrap()
}

#[test]
fn protected_loadout_floor_excludes_equipped_parts() {
    let (data, session) = setup();
    // The starter's spark coil is equipped, so it can't be staked.
    let eligible = eligible_stakes(&session, &data, 0);
    let equipped = session.profile.equipped_item_ids();
    assert!(eligible.iter().all(|id| !equipped.contains(id)));
    // The unequipped healing pod qualifies once value allows.
    assert!(!eligible.is_empty());
    // A sky-high floor filters everything out.
    assert!(eligible_stakes(&session, &data, 100_000).is_empty());
}

#[test]
fn winning_a_staked_duel_transfers_the_stake_and_rank() {
    let (data, mut session) = setup();
    let bruk = duelist(&data, "fernhollow", "bruk").clone();
    let items_before = session.profile.inventory.items.len();
    let scrip_before = session.profile.inventory.scrip;

    let pending = PendingDuel {
        settlement_id: "fernhollow".to_owned(),
        duelist_id: "bruk".to_owned(),
        my_stake: None,
    };
    let lines = apply_duel_result(&mut session, &data, &pending, &bruk, true);

    assert_eq!(session.profile.inventory.items.len(), items_before + 1);
    assert_eq!(
        session.profile.inventory.scrip,
        scrip_before + bruk.reward_scrip
    );
    assert_eq!(current_rank(&session, "fernhollow"), bruk.rank_req + 1);
    assert!(lines.iter().any(|l| l.contains("Ember Spitter")));
}

#[test]
fn losing_a_staked_duel_forfeits_your_part() {
    let (data, mut session) = setup();
    let bruk = duelist(&data, "fernhollow", "bruk").clone();
    let staked = session
        .profile
        .grant_graft("bolt_cannon", GraftCondition::Intact);
    let items_before = session.profile.inventory.items.len();

    let pending = PendingDuel {
        settlement_id: "fernhollow".to_owned(),
        duelist_id: "bruk".to_owned(),
        my_stake: Some(staked),
    };
    apply_duel_result(&mut session, &data, &pending, &bruk, false);

    assert_eq!(session.profile.inventory.items.len(), items_before - 1);
    assert!(session.profile.inventory.item(staked).is_none());
    assert_eq!(current_rank(&session, "fernhollow"), 0);
}

#[test]
fn rank_gates_challenges() {
    let (data, mut session) = setup();
    let orla = duelist(&data, "fernhollow", "orla").clone();
    assert!(!can_challenge(&session, "fernhollow", &orla));
    session
        .profile
        .rider
        .duel_ranks
        .insert("fernhollow".to_owned(), 1);
    assert!(can_challenge(&session, "fernhollow", &orla));
}

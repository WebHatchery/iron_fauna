use crate::data::GameData;

/// Every quest must reference real reward grafts and a sane objective.
#[test]
fn quests_are_valid() {
    let data = GameData::load().unwrap();
    assert!(!data.quests.is_empty(), "no quests authored");
    for (_, q) in data.quests.iter() {
        assert!(q.objective.count > 0, "{}: zero-count objective", q.id);
        assert!(!q.tracker.is_empty(), "{}: empty tracker", q.id);
        for graft in &q.reward_grafts {
            assert!(
                data.graftware.contains(graft),
                "{}: unknown reward graft {}",
                q.id,
                graft
            );
        }
    }
}

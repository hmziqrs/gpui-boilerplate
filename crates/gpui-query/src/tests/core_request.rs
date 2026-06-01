use crate::core::*;

#[test]
fn request_sequencer_allocates_monotonic_request_ids() {
    let mut sequencer = RequestSequencer::new();

    assert_eq!(sequencer.next_request().value(), 1);
    assert_eq!(sequencer.next_request().value(), 2);
}

#[test]
fn request_sequencer_scope_changes_make_old_request_ids_stale() {
    let mut sequencer = RequestSequencer::new();
    let old_request = sequencer.next_request();

    sequencer.advance_scope();
    let new_request = sequencer.next_request();

    assert_ne!(old_request, new_request);
    assert!(!sequencer.is_current_scope(old_request));
    assert!(sequencer.is_current_scope(new_request));
}

#[test]
fn request_sequencer_advances_scope_on_sequence_overflow() {
    let mut sequencer = RequestSequencer {
        scope_id: 7,
        next_request_id: u64::MAX,
    };

    let max_request = sequencer.next_request();
    let next_request = sequencer.next_request();

    assert_eq!(max_request, RequestId::scoped(7, u64::MAX));
    assert_eq!(next_request, RequestId::scoped(8, 1));
}

use super::*;

fn sample_entry() -> UndoEntry {
    UndoEntry {
        label: "Switch Theme".to_string(),
        undo_label: "Undo Theme Switch".to_string(),
        redo_label: "Redo Theme Switch".to_string(),
        created_at: AppTimestamp::now(),
        kind: UndoKind::ThemeMode {
            before: ThemeMode::Light,
            after: ThemeMode::Dark,
        },
    }
}

#[test]
fn record_clears_redo_history() {
    let mut model = UndoModel {
        future: vec![sample_entry()],
        ..UndoModel::default()
    };
    model.record(sample_entry());
    assert_eq!(model.past.len(), 1);
    assert!(model.future.is_empty());
}

#[test]
fn pop_undo_sets_rejected_reason_when_empty() {
    let mut model = UndoModel::default();
    assert!(model.pop_undo().is_none());
    assert_eq!(model.last_rejected.as_deref(), Some("nothing to undo"));
}

#[test]
fn pop_redo_sets_rejected_reason_when_empty() {
    let mut model = UndoModel::default();
    assert!(model.pop_redo().is_none());
    assert_eq!(model.last_rejected.as_deref(), Some("nothing to redo"));
}

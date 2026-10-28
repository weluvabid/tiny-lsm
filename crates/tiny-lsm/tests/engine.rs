use std::collections::BTreeMap;

use tiny_lsm::Engine;

#[test]
fn put_get_overwrite_and_delete() {
    let mut engine = Engine::new();

    assert_eq!(engine.get("missing"), None);
    engine.put("key", "first");
    assert_eq!(engine.get("key").map(|v| v.as_bytes()), Some(&b"first"[..]));

    engine.put("key", "second");
    assert_eq!(
        engine.get("key").map(|v| v.as_bytes()),
        Some(&b"second"[..])
    );

    assert!(engine.delete("key"));
    assert_eq!(engine.get("key"), None);
    assert!(!engine.delete("key"));

    engine.put("key", "again");
    assert_eq!(engine.get("key").map(|v| v.as_bytes()), Some(&b"again"[..]));
}

#[test]
fn empty_key_and_value_are_supported() {
    let mut engine = Engine::new();

    engine.put(Vec::<u8>::new(), Vec::<u8>::new());
    assert_eq!(
        engine.get(Vec::<u8>::new()).map(|v| v.as_bytes()),
        Some(&[][..])
    );
    assert_eq!(engine.len(), 1);

    assert!(engine.delete(Vec::<u8>::new()));
    assert!(engine.is_empty());
}

#[test]
fn len_tracks_live_keys_not_tombstones() {
    let mut engine = Engine::new();

    assert!(engine.is_empty());
    engine.put("a", "1");
    engine.put("b", "2");
    engine.put("a", "3");
    assert_eq!(engine.len(), 2);

    engine.delete("a");
    assert_eq!(engine.len(), 1);
    engine.delete("unknown");
    assert_eq!(engine.len(), 1);

    engine.delete("b");
    assert_eq!(engine.len(), 0);
    assert!(engine.is_empty());
}

#[test]
fn deterministic_operation_sequence_matches_reference_map() {
    let mut engine = Engine::new();
    let mut reference = BTreeMap::<Vec<u8>, Vec<u8>>::new();

    for step in 0_u32..2_000 {
        let key = format!("key-{}", step.wrapping_mul(37) % 53).into_bytes();

        match step % 5 {
            0 | 1 | 4 => {
                let value = format!("value-{step}").into_bytes();
                engine.put(key.clone(), value.clone());
                reference.insert(key, value);
            }
            2 => {
                let expected = reference.remove(&key).is_some();
                assert_eq!(engine.delete(key.clone()), expected);
            }
            _ => {
                assert_eq!(
                    engine
                        .get(key.as_slice())
                        .map(|value| value.as_bytes().to_vec()),
                    reference.get(&key).cloned()
                );
            }
        }

        assert_eq!(engine.len(), reference.len());
        for (key, expected) in &reference {
            assert_eq!(
                engine.get(key.as_slice()).map(|value| value.as_bytes()),
                Some(expected.as_slice())
            );
        }
    }
}

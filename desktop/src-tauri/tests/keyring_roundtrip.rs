//! Explicit OS-store integration test. Contains a synthetic value, never an API key.
//! CI runs this only with a working, disposable OS keyring session.

#[test]
#[ignore = "requires an unlocked native OS keyring; opt in explicitly"]
fn native_keyring_roundtrip() {
    let account = format!("synthetic-probe-{}", std::process::id());
    let entry = keyring::Entry::new("dk.justservices.kvoteven.selftest", &account)
        .expect("OS keyring entry creation failed");
    struct Cleanup(keyring::Entry);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = self.0.delete_credential();
        }
    }
    let entry = Cleanup(entry);
    let synthetic = "synthetic-selftest-value-not-a-provider-key";
    entry
        .0
        .set_password(synthetic)
        .expect("OS keyring write failed");
    let actual = zeroize::Zeroizing::new(entry.0.get_password().expect("OS keyring read failed"));
    assert!(
        actual.as_str() == synthetic,
        "OS keyring did not preserve the synthetic value"
    );
    entry
        .0
        .delete_credential()
        .expect("OS keyring delete failed");
    assert!(
        matches!(entry.0.get_password(), Err(keyring::Error::NoEntry)),
        "OS keyring deletion was not confirmed"
    );
}

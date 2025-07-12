use grok_code::keystore::KeyStore;

#[test]
fn test_keystore_basic_operations() {
    let keystore = KeyStore::new();
    let test_provider = "test_provider";
    let test_key = "test_api_key_12345";

    // Clean up any existing key
    let _ = keystore.delete_api_key(test_provider);

    // Test that key doesn't exist initially
    assert!(!keystore.has_api_key(test_provider));

    // Test storing a key
    match keystore.set_api_key(test_provider, test_key) {
        Ok(_) => {
            // Test retrieving the key
            match keystore.get_api_key(test_provider) {
                Ok(retrieved_key) => assert_eq!(retrieved_key, test_key),
                Err(e) => panic!("Failed to retrieve key: {}", e),
            }

            // Test that key exists
            assert!(keystore.has_api_key(test_provider));

            // Test deleting the key
            match keystore.delete_api_key(test_provider) {
                Ok(_) => {
                    // Test that key no longer exists
                    assert!(!keystore.has_api_key(test_provider));
                }
                Err(e) => panic!("Failed to delete key: {}", e),
            }
        }
        Err(e) => {
            // Keyring might not be available in CI environment
            eprintln!(
                "Warning: Keystore test skipped - keyring not available: {}",
                e
            );
        }
    }
}

#[test]
fn test_keystore_multiple_providers() {
    let keystore = KeyStore::new();

    // Try to set keys for different providers
    let providers = vec![
        ("xai", "xai_test_key_123"),
        ("openai", "openai_test_key_456"),
    ];

    // Clean up first
    for (provider, _) in &providers {
        let _ = keystore.delete_api_key(provider);
    }

    // Try to store keys
    let mut stored_count = 0;
    for (provider, key) in &providers {
        match keystore.set_api_key(provider, key) {
            Ok(_) => stored_count += 1,
            Err(e) => eprintln!("Warning: Could not store key for {}: {}", provider, e),
        }
    }

    // If we managed to store keys, test retrieval
    if stored_count > 0 {
        for (provider, expected_key) in &providers {
            if keystore.has_api_key(provider) {
                match keystore.get_api_key(provider) {
                    Ok(key) => assert_eq!(key, *expected_key),
                    Err(e) => panic!("Failed to retrieve key for {}: {}", provider, e),
                }
            }
        }

        // Clean up
        for (provider, _) in &providers {
            let _ = keystore.delete_api_key(provider);
        }
    }
}

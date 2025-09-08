//! Test HUGR 0.13 support in pecos-selene

#[cfg(feature = "hugr-013")]
#[test]
fn test_hugr_013_in_selene() {
    use pecos_selene::hugr_013_support;

    // This test verifies that HUGR 0.13 types are available
    // The actual Package type comes from hugr-core 0.13
    let json_data = r#"{
        "modules": [],
        "extensions": []
    }"#;

    // Try to load a minimal HUGR 0.13 package
    let result = hugr_013_support::load_hugr_013_package(json_data.as_bytes());

    match result {
        Ok(package) => {
            println!("Successfully loaded HUGR 0.13 package");
            assert_eq!(package.modules.len(), 0);
            assert_eq!(package.extensions.len(), 0);
        }
        Err(e) => {
            panic!("Failed to load HUGR 0.13 package: {e}");
        }
    }
}

#[test]
fn test_hugr_version_info() {
    println!("pecos-selene uses HUGR 0.13 for guppylang compatibility");
    println!("This allows loading HUGR with List types from guppylang");

    // Verify the feature is enabled by default
    #[cfg(feature = "hugr-013")]
    {
        println!("✓ HUGR 0.13 support is enabled");
    }

    #[cfg(not(feature = "hugr-013"))]
    {
        panic!("HUGR 0.13 support should be enabled by default!");
    }
}

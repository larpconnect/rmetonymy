use ipa::IpaSystem;

#[test]
fn test_full_database_loads_without_errors() {
    let system = IpaSystem::default();

    // Check for some pulmonic consonants
    assert!(system.get_phoneme_data("p").is_some());
    assert!(system.get_phoneme_data("b").is_some());
    assert!(system.get_phoneme_data("k").is_some());
    assert!(system.get_phoneme_data("ɡ").is_some());
    assert!(system.get_phoneme_data("m").is_some());
    assert!(system.get_phoneme_data("n").is_some());

    // Check non-pulmonic consonants
    assert!(system.get_phoneme_data("ʘ").is_some()); // click
    assert!(system.get_phoneme_data("ɓ").is_some()); // implosive
    assert!(system.get_phoneme_data("pʼ").is_some()); // ejective

    // Check vowels
    assert!(system.get_phoneme_data("i").is_some());
    assert!(system.get_phoneme_data("a").is_some());
    assert!(system.get_phoneme_data("u").is_some());
}

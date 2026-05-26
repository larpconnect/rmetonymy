use ipa::IpaString;
use language::dictionary::{Dictionary, NewEntry, generate_base62_uuid, parse_base62_uuid};
use std::collections::BTreeMap;
use std::str::FromStr;
use uuid::Uuid;

#[test]
fn test_base62_uuid_roundtrip() {
    let original = Uuid::now_v7();
    let encoded = base62::encode(original.as_u128());
    let decoded = parse_base62_uuid(&encoded).expect("parse base62 uuid");
    assert_eq!(original, decoded);
}

#[test]
fn test_generate_base62_uuid() {
    let encoded = generate_base62_uuid();
    assert!(!encoded.is_empty());
    let decoded = parse_base62_uuid(&encoded).expect("parse generated base62 uuid");
    assert_eq!(decoded.get_version(), Some(uuid::Version::SortRand)); // UUIDv7
}

#[test]
fn test_dictionary_blank_and_add_remove() {
    let lang_id = Uuid::now_v7();
    let mut dict = Dictionary::new(lang_id);
    assert_eq!(dict.id, lang_id);
    assert!(dict.entries.is_empty());

    let meaning = IpaString::from_str("rɛd").expect("parse meaning");
    let definition = IpaString::from_str("pat").expect("parse definition");
    let r#type = "noun.masculine".to_string();
    let era = Some(1);
    let mut etymology = BTreeMap::new();
    etymology.insert(0, vec!["pa".to_string(), "ta".to_string()]);
    let usage_notes = "Used primarily in formal contexts.".to_string();

    let entry_id = dict.add_entry(NewEntry {
        meaning: meaning.clone(),
        definition: definition.clone(),
        r#type: r#type.clone(),
        era,
        etymology: Some(etymology.clone()),
        usage_notes: usage_notes.clone(),
    });

    assert_eq!(dict.entries.len(), 1);
    let entry = dict.entries.first().expect("entry 0 exists");
    assert_eq!(entry.id, entry_id);
    assert_eq!(entry.meaning, meaning);
    assert_eq!(entry.definition, definition);
    assert_eq!(entry.r#type, r#type);
    assert_eq!(entry.era, 1);
    assert_eq!(entry.etymology, Some(etymology));
    assert_eq!(entry.usage_notes, usage_notes);

    // Test removing entry
    let removed = dict.remove_entry(&entry_id);
    assert!(removed);
    assert!(dict.entries.is_empty());

    let removed_again = dict.remove_entry(&entry_id);
    assert!(!removed_again);
}

#[test]
fn test_dictionary_serialization_validation() {
    let lang_id = Uuid::parse_str("018f4a3e-6b9f-7a1a-9b1a-2b3c4d5e6f7a").expect("valid UUID");
    let mut dict = Dictionary::new(lang_id);

    let meaning = IpaString::from_str("fʊd").expect("valid IPA string");
    let definition = IpaString::from_str("kam").expect("valid IPA string");
    let mut etymology = BTreeMap::new();
    etymology.insert(1, vec!["ka".to_string()]);

    dict.add_entry(NewEntry {
        meaning,
        definition,
        r#type: "noun.neuter".to_string(),
        era: Some(2),
        etymology: Some(etymology),
        usage_notes: "Common word.".to_string(),
    });

    let json_str = dict.to_string().expect("serialize dict");
    let parsed_dict = Dictionary::from_str(&json_str).expect("parse and validate dict");
    assert_eq!(dict, parsed_dict);
}

#[test]
fn test_dictionary_optional_etymology() {
    let lang_id = Uuid::now_v7();
    let mut dict = Dictionary::new(lang_id);

    dict.add_entry(NewEntry {
        meaning: IpaString::from_str("a").expect("valid IPA string"),
        definition: IpaString::from_str("b").expect("valid IPA string"),
        r#type: "noun".to_string(),
        era: Some(1),
        etymology: None,
        usage_notes: "Notes".to_string(),
    });

    let json_str = dict.to_string().expect("serialize dict");
    // Assert that "etymology" key is not present in the serialized JSON string
    assert!(!json_str.contains("etymology"));

    let parsed_dict = Dictionary::from_str(&json_str).expect("parse dict without etymology");
    assert_eq!(parsed_dict.entries.first().expect("entry 0 exists").etymology, None);
}

#[test]
fn test_dictionary_default_era() {
    let lang_id = Uuid::now_v7();
    let mut dict = Dictionary::new(lang_id);

    // 1. Era defaults to 0 when dictionary is empty
    dict.add_entry(NewEntry {
        meaning: IpaString::from_str("a").expect("valid IPA string"),
        definition: IpaString::from_str("b").expect("valid IPA string"),
        r#type: "noun".to_string(),
        era: None,
        etymology: None,
        usage_notes: String::new(),
    });
    assert_eq!(dict.entries.first().expect("entry 0 exists").era, 0);

    // 2. Add entry with explicit era 3
    dict.add_entry(NewEntry {
        meaning: IpaString::from_str("c").expect("valid IPA string"),
        definition: IpaString::from_str("d").expect("valid IPA string"),
        r#type: "verb".to_string(),
        era: Some(3),
        etymology: None,
        usage_notes: String::new(),
    });
    assert_eq!(dict.entries.get(1).expect("entry 1 exists").era, 3);

    // 3. Era defaults to 3 (most recent era present in the dictionary)
    dict.add_entry(NewEntry {
        meaning: IpaString::from_str("e").expect("valid IPA string"),
        definition: IpaString::from_str("f").expect("valid IPA string"),
        r#type: "adjective".to_string(),
        era: None,
        etymology: None,
        usage_notes: String::new(),
    });
    assert_eq!(dict.entries.get(2).expect("entry 2 exists").era, 3);
}

#[test]
fn test_dictionary_validation_fails_on_invalid_json() {
    // Missing required fields (like entries or id)
    let invalid_json1 = r#"{
        "id": "018f4a3e-6b9f-7a1a-9b1a-2b3c4d5e6f7a"
    }"#;
    let res1 = Dictionary::from_str(invalid_json1);
    assert!(res1.is_err());
    assert!(res1.expect_err("error exists").contains("Schema validation failed"));

    // Incorrect type for era (should be integer, not string)
    let invalid_json2 = r#"{
        "id": "018f4a3e-6b9f-7a1a-9b1a-2b3c4d5e6f7a",
        "entries": [
            {
                "id": "3gS5H3f",
                "meaning": "a",
                "definition": "b",
                "type": "noun.masc",
                "era": "one",
                "usage_notes": ""
            }
        ]
    }"#;
    let res2 = Dictionary::from_str(invalid_json2);
    assert!(res2.is_err());
    assert!(res2.expect_err("error exists").contains("Schema validation failed"));
}

#[test]
fn test_atomic_write() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let file_path = temp_dir.path().join("test_dict.json");
    let data = b"some data to write atomically";

    language::dictionary::atomic_write(&file_path, data).expect("atomic write");
    let read_data = std::fs::read(&file_path).expect("read file");
    assert_eq!(read_data, data);
}

#[test]
fn test_dictionary_add_era() {
    let lang_id = Uuid::now_v7();
    let mut dict = Dictionary::new(lang_id);

    // 1. Add era with implicit/default era number (should start at 0)
    let (era_num1, era_id1) = dict
        .add_era(None, None, Some("First era".to_string()))
        .expect("add era 1");
    assert_eq!(era_num1, 0);
    assert!(!era_id1.is_empty());

    // 2. Add era with explicit era number
    let (era_num2, era_id2) = dict
        .add_era(Some(3), Some(IpaString::from_str("a").expect("valid IPA string")), None)
        .expect("add era 2");
    assert_eq!(era_num2, 3);
    assert!(!era_id2.is_empty());

    // 3. Add era with implicit era number (should be max + 1 = 4)
    let (era_num3, era_id3) = dict
        .add_era(None, None, None)
        .expect("add era 3");
    assert_eq!(era_num3, 4);
    assert!(!era_id3.is_empty());

    // 4. Try adding an era that already exists
    let err = dict.add_era(Some(3), None, None);
    assert!(err.is_err());
    assert!(err.expect_err("should fail to add duplicate era").contains("already exists"));

    // Verify eras map structure
    assert_eq!(dict.eras.len(), 3);
    assert_eq!(dict.eras.get(&0).expect("era 0 exists").description, Some("First era".to_string()));
    assert_eq!(
        dict.eras.get(&3).expect("era 3 exists").name,
        Some(IpaString::from_str("a").expect("valid IPA string"))
    );
    assert_eq!(dict.eras.get(&4).expect("era 4 exists").description, None);
}

#[test]
fn test_dictionary_auto_create_era() {
    let lang_id = Uuid::now_v7();
    let mut dict = Dictionary::new(lang_id);

    // 1. Add entry with an explicit era that does not exist. It should be auto-created.
    dict.add_entry(NewEntry {
        meaning: IpaString::from_str("a").expect("valid IPA string"),
        definition: IpaString::from_str("b").expect("valid IPA string"),
        r#type: "noun".to_string(),
        era: Some(5),
        etymology: None,
        usage_notes: String::new(),
    });

    assert_eq!(dict.eras.len(), 1);
    assert!(dict.eras.contains_key(&5));
    assert!(!dict.eras.get(&5).expect("era 5 exists").id.is_empty());
    assert_eq!(dict.eras.get(&5).expect("era 5 exists").name, None);
    assert_eq!(dict.eras.get(&5).expect("era 5 exists").description, None);

    // 2. Add entry with no era. It should default to 5 (the max era in the eras map).
    dict.add_entry(NewEntry {
        meaning: IpaString::from_str("c").expect("valid IPA string"),
        definition: IpaString::from_str("d").expect("valid IPA string"),
        r#type: "verb".to_string(),
        era: None,
        etymology: None,
        usage_notes: String::new(),
    });

    assert_eq!(dict.entries.get(1).expect("entry 1 exists").era, 5);
}

#[test]
fn test_dictionary_era_serialization() {
    let lang_id = Uuid::now_v7();
    let mut dict = Dictionary::new(lang_id);

    dict.add_era(Some(1), Some(IpaString::from_str("e").expect("valid IPA string")), Some("desc".to_string()))
        .expect("add era");

    let json_str = dict.to_string().expect("serialize dict");
    assert!(json_str.contains(r#""eras""#));
    assert!(json_str.contains(r#""name": "e""#));
    assert!(json_str.contains(r#""description": "desc""#));

    let parsed = Dictionary::from_str(&json_str).expect("parse back");
    assert_eq!(parsed.eras.len(), 1);
    assert_eq!(parsed.eras.get(&1).expect("era 1 exists").description, Some("desc".to_string()));
}

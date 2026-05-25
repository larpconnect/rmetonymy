use ipa::IpaString;
use language::dictionary::{
    generate_base62_uuid, parse_base62_uuid, Dictionary, NewEntry,
};
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
    let word_type = "noun".to_string();
    let word_subtype = "masculine".to_string();
    let era = 1;
    let mut etymology = BTreeMap::new();
    etymology.insert(0, vec!["pa".to_string(), "ta".to_string()]);
    let usage_notes = "Used primarily in formal contexts.".to_string();

    let entry_id = dict.add_entry(NewEntry {
        meaning: meaning.clone(),
        definition: definition.clone(),
        word_type: word_type.clone(),
        word_subtype: word_subtype.clone(),
        era,
        etymology: etymology.clone(),
        usage_notes: usage_notes.clone(),
    });

    assert_eq!(dict.entries.len(), 1);
    let entry = &dict.entries[0];
    assert_eq!(entry.id, entry_id);
    assert_eq!(entry.meaning, meaning);
    assert_eq!(entry.definition, definition);
    assert_eq!(entry.word_type, word_type);
    assert_eq!(entry.word_subtype, word_subtype);
    assert_eq!(entry.era, era);
    assert_eq!(entry.etymology, etymology);
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
    let lang_id = Uuid::parse_str("018f4a3e-6b9f-7a1a-9b1a-2b3c4d5e6f7a").unwrap();
    let mut dict = Dictionary::new(lang_id);

    let meaning = IpaString::from_str("fʊd").unwrap();
    let definition = IpaString::from_str("kam").unwrap();
    let mut etymology = BTreeMap::new();
    etymology.insert(1, vec!["ka".to_string()]);

    dict.add_entry(NewEntry {
        meaning,
        definition,
        word_type: "noun".to_string(),
        word_subtype: "neuter".to_string(),
        era: 2,
        etymology,
        usage_notes: "Common word.".to_string(),
    });

    let json_str = dict.to_string().expect("serialize dict");
    let parsed_dict = Dictionary::from_str(&json_str).expect("parse and validate dict");
    assert_eq!(dict, parsed_dict);
}

#[test]
fn test_dictionary_validation_fails_on_invalid_json() {
    // Missing required fields (like entries or id)
    let invalid_json1 = r#"{
        "id": "018f4a3e-6b9f-7a1a-9b1a-2b3c4d5e6f7a"
    }"#;
    let res1 = Dictionary::from_str(invalid_json1);
    assert!(res1.is_err());
    assert!(res1.err().unwrap().contains("Schema validation failed"));

    // Incorrect type for era (should be integer, not string)
    let invalid_json2 = r#"{
        "id": "018f4a3e-6b9f-7a1a-9b1a-2b3c4d5e6f7a",
        "entries": [
            {
                "id": "3gS5H3f",
                "meaning": "a",
                "definition": "b",
                "word_type": "noun",
                "word_subtype": "masc",
                "era": "one",
                "etymology": {},
                "usage_notes": ""
            }
        ]
    }"#;
    let res2 = Dictionary::from_str(invalid_json2);
    assert!(res2.is_err());
    assert!(res2.err().unwrap().contains("Schema validation failed"));
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

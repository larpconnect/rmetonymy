use proptest::prelude::*;

proptest! {
    #[test]
    fn dummy_proptest(i in 0..100i32) {
        assert!(i >= 0 && i < 100);
    }
}

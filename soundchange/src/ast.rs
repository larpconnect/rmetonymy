use data::feature::Feature;
use ipa::IpaString;

pub use language::{EraRules, PreambleItem, PreambleType, SoundChangeRule, SoundChanges};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    RightMultipleTransparent, // => or >
    RightSingleTransparent,   // ->
    RightMultipleOpaque,      // =:>
    LeftMultipleTransparent,  // <= or <
    LeftSingleTransparent,    // <-
    LeftMultipleOpaque,       // <:=
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedMatchPart {
    Pattern(MatchPattern),
    Reference(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedTransformPart {
    Pattern(TransformPattern),
    Reference(String),
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedSoundChange {
    Reference(String),
    Rule {
        match_part: Option<ParsedMatchPart>,
        operator: Operator,
        transform_part: Option<ParsedTransformPart>,
        condition: Option<ConditionExpr>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchPattern {
    pub elements: Vec<MatchElement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchElement {
    pub base: MatchBase,
    pub modifiers_wildcard: bool, // true if followed by ᴴ
    pub quantifier: MatchQuantifier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchBase {
    WordBoundary,
    SyllableBoundary, // $
    SoundClass {
        key: language::sound_class::SoundClassKey,
        marker: Option<u8>,
    },
    SetExclusion {
        key: language::sound_class::SoundClassKey,
        marker: Option<u8>,
    },
    IpaSequence(IpaString),
    FeatureClass {
        key_opt: Option<FeatureClassKey>,
        features: Vec<FeatureDescriptor>,
    },
    Set(Vec<MatchBase>),
    OptionalGroup(MatchPattern),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureClassKey {
    pub key: Option<language::sound_class::SoundClassKey>,
    pub exclude: bool,
    pub marker: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchQuantifier {
    None,
    ZeroOrMore,
    OneOrMore,
    ZeroOrMoreBounded(u32),
    OneOrMoreBounded(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureDescriptor {
    pub sign: bool, // true for +, false for -
    pub alpha: Option<AlphaVariable>,
    pub feature: Feature,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlphaVariable {
    pub greek: char,
    pub name: String,
    pub sign: bool, // true if alpha has a leading "-"
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransformPattern {
    pub elements: Vec<TransformElement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransformElement {
    Empty, // ∅
    Literal {
        ipa: IpaString,
        copy_modifiers: bool,
        append_modifiers: Vec<String>,
    },
    Ref {
        marker: Option<u8>,
        class_key: Option<language::sound_class::SoundClassKey>,
        repeat: usize,
        copy_modifiers: bool,
        append_modifiers: Vec<String>,
        feature_changes: Vec<FeatureDescriptor>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionExpr {
    Reference(String),
    Term {
        negated: bool,
        pattern: ConditionPattern,
    },
    Binary {
        left: Box<ConditionExpr>,
        op: ConditionOp,
        right: Box<ConditionExpr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionOp {
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionPattern {
    pub elements: Vec<ConditionElement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionElement {
    pub base: ConditionBase,
    pub quantifier: MatchQuantifier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionBase {
    MatchPlaceholder, // _
    Element(MatchBase),
}

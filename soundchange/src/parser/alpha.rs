use crate::ast::AlphaVariable;
use crate::parser::Rule;
use pest::iterators::Pair;

pub(crate) fn convert_alpha_variable(pair: Pair<'_, Rule>) -> AlphaVariable {
    let mut sign = false;
    let mut greek = 'α';
    let mut name = String::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::feature_sign => {
                sign = inner.as_str() == "-";
            }
            Rule::greek_letter => {
                greek = inner.as_str().chars().next().unwrap_or('α');
            }
            Rule::name => {
                name = inner.as_str().to_string();
            }
            _ => {}
        }
    }

    AlphaVariable { greek, name, sign }
}

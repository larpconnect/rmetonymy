use crate::ast::FeatureDescriptor;
use crate::evaluator::features::get_phoneme_features_map;
use crate::evaluator::{CapturedAlpha, EvalContext, MatchState};
use data::feature::Feature;
use ipa::sequence::Phoneme;

fn apply_place_manner_change_op<'a>(
    fd: &FeatureDescriptor,
    state: &'a MatchState,
    current_place: &'a [String],
    current_manner: &'a [String],
) -> (&'a [String], &'a [String]) {
    let mut place = current_place;
    let mut manner = current_manner;
    if let Some(CapturedAlpha::Strings(s)) = fd
        .alpha
        .as_ref()
        .and_then(|alpha| state.alpha.get(&alpha.name))
    {
        if fd.feature == Feature::Place {
            place = s.as_slice();
        } else {
            manner = s.as_slice();
        }
    }
    (place, manner)
}

fn evaluate_feature_change_sign_op(fd: &FeatureDescriptor, state: &MatchState) -> bool {
    super::descriptor::evaluate_descriptor_sign_op(fd, state)
}

pub(crate) fn apply_feature_changes(
    p: &Phoneme,
    changes: &[FeatureDescriptor],
    state: &MatchState,
    ctx: &EvalContext<'_>,
) -> Result<Phoneme, String> {
    let mut map = get_phoneme_features_map(p);
    let mut target_place = if let Some(d) = ipa::get_phoneme_data(&p.base) {
        d.place.as_slice()
    } else {
        &[]
    };
    let mut target_manner = if let Some(d) = ipa::get_phoneme_data(&p.base) {
        d.manner.as_slice()
    } else {
        &[]
    };

    for fd in changes {
        if fd.feature == Feature::Stress {
            continue;
        }
        if fd.feature == Feature::Place || fd.feature == Feature::Manner {
            let (p, m) = apply_place_manner_change_op(fd, state, target_place, target_manner);
            target_place = p;
            target_manner = m;
            continue;
        }
        let sign = evaluate_feature_change_sign_op(fd, state);
        map.insert(fd.feature, sign);
    }

    let best_base = super::helper::find_best_phoneme_base(&map, target_place, target_manner, ctx)?;
    Ok(Phoneme {
        base: best_base,
        modifiers: p.modifiers.clone(),
    })
}

use crate::ast::FeatureDescriptor;
use crate::evaluator::{CapturedAlpha, EvalContext, MatchState};
use crate::evaluator::features::get_phoneme_features_map;
use data::feature::Feature;
use ipa::sequence::Phoneme;

pub(crate) fn apply_feature_changes(
    p: &Phoneme,
    changes: &[FeatureDescriptor],
    state: &MatchState,
    ctx: &EvalContext<'_>,
) -> Result<Phoneme, String> {
    let mut map = get_phoneme_features_map(p);
    let mut target_place = if let Some(d) = ipa::get_phoneme_data(&p.base) {
        d.place.clone()
    } else {
        Vec::new()
    };
    let mut target_manner = if let Some(d) = ipa::get_phoneme_data(&p.base) {
        d.manner.clone()
    } else {
        Vec::new()
    };

    for fd in changes {
        if fd.feature == Feature::Stress {
            continue;
        }
        if fd.feature == Feature::Place || fd.feature == Feature::Manner {
            if let Some(CapturedAlpha::Strings(s)) = fd
                .alpha
                .as_ref()
                .and_then(|alpha| state.alpha.get(&alpha.name))
            {
                if fd.feature == Feature::Place {
                    target_place.clone_from(s);
                } else {
                    target_manner.clone_from(s);
                }
            }
            continue;
        }
        let sign = if let Some(ref alpha) = fd.alpha {
            match state.alpha.get(&alpha.name) {
                Some(CapturedAlpha::Sign(s)) => {
                    if alpha.sign {
                        !s
                    } else {
                        *s
                    }
                }
                _ => false,
            }
        } else {
            fd.sign
        };
        map.insert(fd.feature, sign);
    }

    let best_base = super::helper::find_best_phoneme_base(&map, &target_place, &target_manner, ctx)?;
    Ok(Phoneme {
        base: best_base,
        modifiers: p.modifiers.clone(),
    })
}

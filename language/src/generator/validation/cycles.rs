use crate::config::SoundClass;
use crate::generator::WordGenerator;
use crate::sound_class::SoundClassKey;
use std::collections::{BTreeMap, HashSet};
use super::{ValidationError, collect_pattern_references, resolve_generator_key};

/// Validates that there are no circular containment relationships in sound classes.
///
/// # Errors
/// Returns `Err` if any containment cycle is detected.
pub fn validate_sound_class_cycles(
    sound_classes: &BTreeMap<SoundClassKey, SoundClass>,
) -> Result<(), ValidationError> {
    let graph = build_sound_class_graph_op(sound_classes);
    match has_cycle_op(&graph) {
        Some(node) => Err(ValidationError::CircularSoundClassContainment(node.to_string())),
        None => Ok(()),
    }
}

fn build_sound_class_graph_op(
    sound_classes: &BTreeMap<SoundClassKey, SoundClass>,
) -> BTreeMap<SoundClassKey, Vec<SoundClassKey>> {
    let mut graph = BTreeMap::new();
    for (key, sc) in sound_classes {
        let mut deps = Vec::new();
        for val in &sc.values {
            if let Some(nested_key) = val
                .parse::<SoundClassKey>()
                .ok()
                .filter(|k| sound_classes.contains_key(k))
            {
                deps.push(nested_key);
            }
        }
        graph.insert(key.clone(), deps);
    }
    graph
}

fn has_cycle_op<T: Ord + std::hash::Hash + Clone>(
    graph: &BTreeMap<T, Vec<T>>,
) -> Option<T> {
    let mut visited = HashSet::new();

    for start_node in graph.keys() {
        if visited.contains(start_node) {
            continue;
        }

        let mut stack = vec![(start_node.clone(), 0)];
        let mut visiting = HashSet::new();
        visiting.insert(start_node.clone());

        while let Some((curr, edge_idx)) = stack.pop() {
            if let Some(deps) = graph.get(&curr) {
                if let Some(dep) = deps.get(edge_idx) {
                    stack.push((curr.clone(), edge_idx + 1));

                    if visiting.contains(dep) {
                        return Some(curr.clone());
                    }
                    if !visited.contains(dep) {
                        visiting.insert(dep.clone());
                        stack.push((dep.clone(), 0));
                    }
                } else {
                    visiting.remove(&curr);
                    visited.insert(curr.clone());
                }
            } else {
                visiting.remove(&curr);
                visited.insert(curr.clone());
            }
        }
    }
    None
}

/// Validates that there are no circular generation dependencies.
///
/// # Errors
/// Returns `Err` if any pattern reference cycle is detected.
pub fn validate_generator_cycles(
    generators: &BTreeMap<String, WordGenerator>,
) -> Result<(), ValidationError> {
    let graph = build_generator_graph_integration(generators)?;
    match has_cycle_op(&graph) {
        Some(node) => Err(ValidationError::CircularPatternReferences(node)),
        None => Ok(()),
    }
}

fn build_generator_graph_integration(
    generators: &BTreeMap<String, WordGenerator>,
) -> Result<BTreeMap<String, Vec<String>>, ValidationError> {
    let mut graph = BTreeMap::new();
    for (key, generator) in generators {
        let mut sound_classes = Vec::new();
        let mut grammar_refs = Vec::new();
        for pattern in &generator.patterns {
            collect_pattern_references(pattern, &mut sound_classes, &mut grammar_refs);
        }

        let mut deps = Vec::new();
        for r in &grammar_refs {
            let resolved = resolve_generator_key(r, generators)?;
            deps.push(resolved);
        }

        deps.sort_unstable();
        deps.dedup();
        graph.insert(key.clone(), deps);
    }
    Ok(graph)
}


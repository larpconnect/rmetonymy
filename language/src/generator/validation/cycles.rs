use super::{ValidationError, collect_pattern_references, resolve_generator_key};
use crate::config::SoundClass;
use crate::generator::WordGenerator;
use crate::sound_class::SoundClassKey;
use std::collections::{BTreeMap, HashSet};

/// Validates that there are no circular containment relationships in sound classes.
///
/// # Errors
/// Returns `Err` if any containment cycle is detected.
pub fn validate_sound_class_cycles(
    sound_classes: &BTreeMap<SoundClassKey, SoundClass>,
) -> Result<(), ValidationError> {
    let graph = build_sound_class_graph_op(sound_classes);
    match has_cycle_op(&graph) {
        Some(node) => Err(ValidationError::CircularSoundClassContainment(
            node.to_string(),
        )),
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

fn process_stack_op<T: Ord + std::hash::Hash + Clone>(
    graph: &BTreeMap<T, Vec<T>>,
    start: &T,
    visiting: &mut HashSet<T>,
    visited: &mut HashSet<T>,
) -> Option<T> {
    let mut stack = vec![(start.clone(), 0)];
    visiting.insert(start.clone());
    while let Some((node, idx)) = stack.last_mut() {
        let popped = match graph.get(node) {
            Some(deps) if *idx < deps.len() => {
                let dep = &deps[*idx];
                *idx += 1;
                if visiting.contains(dep) {
                    return Some(node.clone());
                }
                if !visited.contains(dep) {
                    visiting.insert(dep.clone());
                    stack.push((dep.clone(), 0));
                }
                None
            }
            _ => stack.pop(),
        };
        if let Some((p, _)) = popped {
            visiting.remove(&p);
            visited.insert(p);
        }
    }
    None
}

fn has_cycle_op<T: Ord + std::hash::Hash + Clone>(graph: &BTreeMap<T, Vec<T>>) -> Option<T> {
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    for start in graph.keys() {
        if visited.contains(start) {
            continue;
        }
        if let Some(cycle_node) = process_stack_op(graph, start, &mut visiting, &mut visited) {
            return Some(cycle_node);
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

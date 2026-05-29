use crate::config::SoundClass;
use crate::generator::{WordGenerator, WordPattern};
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
                if edge_idx < deps.len() {
                    let dep = &deps[edge_idx];
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
    build_graph_loop_op(
        generators,
        |generator| {
            collect_generator_refs_integration(generator)
        },
        |refs| {
            resolve_refs_integration(refs, generators)
        },
        sort_and_dedup_deps_op,
    )
}

fn build_graph_loop_op<F, G, H>(
    generators: &BTreeMap<String, WordGenerator>,
    mut collect_fn: F,
    mut resolve_fn: G,
    mut sort_fn: H,
) -> Result<BTreeMap<String, Vec<String>>, ValidationError>
where
    F: FnMut(&WordGenerator) -> Vec<String>,
    G: FnMut(&[String]) -> Result<Vec<String>, ValidationError>,
    H: FnMut(Vec<String>) -> Vec<String>,
{
    let mut graph = BTreeMap::new();
    for (key, generator) in generators {
        let grammar_refs = collect_fn(generator);
        let deps = resolve_fn(&grammar_refs)?;
        let sorted_deps = sort_fn(deps);
        graph.insert(key.clone(), sorted_deps);
    }
    Ok(graph)
}

fn collect_generator_refs_integration(
    generator: &WordGenerator,
) -> Vec<String> {
    collect_refs_loop_op(generator, |pattern, sound_classes, grammar_refs| {
        collect_pattern_references(pattern, sound_classes, grammar_refs)
    })
}

fn collect_refs_loop_op<F>(generator: &WordGenerator, mut collect_fn: F) -> Vec<String>
where
    F: FnMut(&WordPattern, &mut Vec<SoundClassKey>, &mut Vec<String>),
{
    let mut sound_classes = Vec::new();
    let mut grammar_refs = Vec::new();
    for pattern in &generator.patterns {
        collect_fn(pattern, &mut sound_classes, &mut grammar_refs);
    }
    grammar_refs
}

fn resolve_refs_integration(
    refs: &[String],
    generators: &BTreeMap<String, WordGenerator>,
) -> Result<Vec<String>, ValidationError> {
    resolve_refs_loop_op(refs, |r| {
        resolve_generator_key(r, generators)
    })
}

fn resolve_refs_loop_op<F>(refs: &[String], mut resolve_fn: F) -> Result<Vec<String>, ValidationError>
where
    F: FnMut(&str) -> Result<String, ValidationError>,
{
    let mut resolved_refs = Vec::new();
    for r in refs {
        let resolved = resolve_fn(r)?;
        resolved_refs.push(resolved);
    }
    Ok(resolved_refs)
}

#[inline]
fn sort_and_dedup_deps_op(mut deps: Vec<String>) -> Vec<String> {
    deps.sort_unstable();
    deps.dedup();
    deps
}

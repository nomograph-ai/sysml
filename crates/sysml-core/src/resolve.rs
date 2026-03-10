use std::collections::{HashMap, HashSet};

use crate::element::SysmlElement;
use crate::relationship::SysmlRelationship;

struct ImportEdge {
    target_namespace: String,
    wildcard: bool,
    segments: Vec<String>,
}

fn build_namespace_map(elements: &[SysmlElement]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for elem in elements {
        if !elem.qualified_name.contains("::")
            && (elem.kind == "package_definition" || elem.kind == "library_package")
        {
            map.insert(
                elem.qualified_name.clone(),
                elem.file_path.to_string_lossy().to_string(),
            );
        }
    }
    map
}

fn build_export_table(elements: &[SysmlElement]) -> HashMap<String, Vec<(String, String)>> {
    let mut by_file: HashMap<String, Vec<&SysmlElement>> = HashMap::new();
    for elem in elements {
        let file = elem.file_path.to_string_lossy().to_string();
        by_file.entry(file).or_default().push(elem);
    }

    let mut table = HashMap::new();
    for (file, elems) in &by_file {
        let ns = elems
            .iter()
            .find(|e| {
                !e.qualified_name.contains("::")
                    && (e.kind == "package_definition" || e.kind == "library_package")
            })
            .map(|e| e.qualified_name.as_str())
            .unwrap_or("");

        let mut exported = Vec::new();
        for elem in elems {
            let qname = &elem.qualified_name;
            let short = qname.rsplit("::").next().unwrap_or(qname);
            let is_top_level = if ns.is_empty() {
                !qname.contains("::")
            } else {
                let prefix = format!("{}::", ns);
                qname.starts_with(&prefix) && !qname[prefix.len()..].contains("::")
            };
            if is_top_level && qname.contains("::") {
                exported.push((short.to_string(), qname.clone()));
            }
        }
        table.insert(file.clone(), exported);
    }
    table
}

fn build_import_graph(relationships: &[SysmlRelationship]) -> HashMap<String, Vec<ImportEdge>> {
    let mut graph: HashMap<String, Vec<ImportEdge>> = HashMap::new();
    for rel in relationships {
        if rel.kind.eq_ignore_ascii_case("import") {
            if let Some(edge) = parse_import_target(&rel.target) {
                let file = rel.file_path.to_string_lossy().to_string();
                graph.entry(file).or_default().push(edge);
            }
        }
    }
    graph
}

fn parse_import_target(target: &str) -> Option<ImportEdge> {
    if target.is_empty() {
        return None;
    }
    let wildcard = target.ends_with("::*");
    let path = if wildcard {
        &target[..target.len() - 3]
    } else {
        target
    };
    let segments: Vec<String> = path.split("::").map(|s| s.to_string()).collect();
    if segments.is_empty() {
        return None;
    }
    Some(ImportEdge {
        target_namespace: segments[0].clone(),
        wildcard,
        segments,
    })
}

fn resolve_file_imports(
    file: &str,
    import_graph: &HashMap<String, Vec<ImportEdge>>,
    namespace_map: &HashMap<String, String>,
    exports: &HashMap<String, Vec<(String, String)>>,
    visible: &mut HashMap<String, String>,
    visited: &mut HashSet<String>,
    elements: &[SysmlElement],
) {
    if visited.contains(file) {
        return;
    }
    visited.insert(file.to_string());

    let edges = match import_graph.get(file) {
        Some(e) => e,
        None => return,
    };

    for edge in edges {
        let target_file = match namespace_map.get(&edge.target_namespace) {
            Some(f) => f.clone(),
            None => continue,
        };

        if edge.segments.len() == 1 && edge.wildcard {
            resolve_file_imports(
                &target_file,
                import_graph,
                namespace_map,
                exports,
                &mut HashMap::new(),
                visited,
                elements,
            );

            if let Some(target_exports) = exports.get(&target_file) {
                for (name, qname) in target_exports {
                    visible.insert(name.clone(), qname.clone());
                }
            }

            let mut transitive = HashMap::new();
            let mut tv = visited.clone();
            resolve_file_imports(
                &target_file,
                import_graph,
                namespace_map,
                exports,
                &mut transitive,
                &mut tv,
                elements,
            );
            for (name, qname) in transitive {
                visible.entry(name).or_insert(qname);
            }
        } else if edge.segments.len() == 1 {
            continue;
        } else {
            let remainder = &edge.segments[1..];
            resolve_multi_segment(
                &target_file,
                remainder,
                edge.wildcard,
                exports,
                elements,
                visible,
            );
        }
    }
}

fn resolve_multi_segment(
    file: &str,
    segments: &[String],
    wildcard: bool,
    exports: &HashMap<String, Vec<(String, String)>>,
    elements: &[SysmlElement],
    visible: &mut HashMap<String, String>,
) {
    let file_elements: Vec<&SysmlElement> = elements
        .iter()
        .filter(|e| e.file_path.to_string_lossy() == file)
        .collect();

    if segments.len() == 1 && !wildcard {
        let name = &segments[0];
        if let Some(file_exports) = exports.get(file) {
            for (ename, qname) in file_exports {
                if ename == name {
                    visible.insert(name.clone(), qname.clone());
                    return;
                }
            }
        }
    } else if segments.len() == 1 && wildcard {
        let container_name = &segments[0];
        for elem in &file_elements {
            let short = elem
                .qualified_name
                .rsplit("::")
                .next()
                .unwrap_or(&elem.qualified_name);
            if short == container_name {
                for member_qname in &elem.members {
                    let member_name = member_qname.rsplit("::").next().unwrap_or(member_qname);
                    visible.insert(member_name.to_string(), member_qname.clone());
                }
                return;
            }
        }
    } else if segments.len() >= 2 {
        let container_name = &segments[0];
        let sub_segments = &segments[1..];

        for elem in &file_elements {
            let short = elem
                .qualified_name
                .rsplit("::")
                .next()
                .unwrap_or(&elem.qualified_name);
            if short == container_name && sub_segments.len() == 1 && !wildcard {
                let target_name = &sub_segments[0];
                for member_qname in &elem.members {
                    let member_name = member_qname.rsplit("::").next().unwrap_or(member_qname);
                    if member_name == target_name {
                        visible.insert(target_name.clone(), member_qname.clone());
                        return;
                    }
                }
            }
        }
    }
}

fn qualify_reference(reference: &str, visible: &HashMap<String, String>) -> String {
    if reference.contains("::") {
        return reference.to_string();
    }
    visible
        .get(reference)
        .cloned()
        .unwrap_or_else(|| reference.to_string())
}

pub fn resolve_imports(elements: &[SysmlElement], relationships: &mut [SysmlRelationship]) {
    let namespace_map = build_namespace_map(elements);
    let exports = build_export_table(elements);
    let import_graph = build_import_graph(relationships);

    let mut files: HashSet<String> = HashSet::new();
    for elem in elements {
        files.insert(elem.file_path.to_string_lossy().to_string());
    }

    let mut resolved: HashMap<String, HashMap<String, String>> = HashMap::new();
    for file in &files {
        let mut visible = HashMap::new();
        let mut visited = HashSet::new();
        resolve_file_imports(
            file,
            &import_graph,
            &namespace_map,
            &exports,
            &mut visible,
            &mut visited,
            elements,
        );
        if !visible.is_empty() {
            resolved.insert(file.clone(), visible);
        }
    }

    for rel in relationships.iter_mut() {
        let file = rel.file_path.to_string_lossy().to_string();
        if let Some(visible) = resolved.get(&file) {
            rel.source = qualify_reference(&rel.source, visible);
            rel.target = qualify_reference(&rel.target, visible);
        }
    }
}

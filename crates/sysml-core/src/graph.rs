use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

use nomograph_core::error::{CoreError, IndexError};
use nomograph_core::traits::KnowledgeGraph;
use nomograph_core::types::{
    CheckType, DetailLevel, Direction, Finding, ParseResult, Predicate, SearchResult, TraceFormat,
    TraceHop, TraceOptions, TraceResult, Triple,
};
use serde::{Deserialize, Serialize};

use crate::element::SysmlElement;
use crate::relationship::SysmlRelationship;
use crate::vocabulary::{expand_query, ExpandedQuery, STRUCTURAL_RELATIONSHIP_KINDS};

#[derive(Default, Serialize, Deserialize)]
pub struct SysmlGraph {
    elements: Vec<SysmlElement>,
    relationships: Vec<SysmlRelationship>,
    #[serde(skip)]
    elements_by_name: HashMap<String, Vec<usize>>,
    #[serde(skip)]
    elements_by_kind: HashMap<String, Vec<usize>>,
    #[serde(skip)]
    rels_by_source: HashMap<String, Vec<usize>>,
    #[serde(skip)]
    rels_by_target: HashMap<String, Vec<usize>>,
    #[cfg(feature = "vector")]
    #[serde(skip)]
    vector_index: Option<crate::vector::VectorIndex>,
}

impl SysmlGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn rebuild_indices(&mut self) {
        self.elements_by_name.clear();
        self.elements_by_kind.clear();
        self.rels_by_source.clear();
        self.rels_by_target.clear();

        for (i, elem) in self.elements.iter().enumerate() {
            self.elements_by_name
                .entry(elem.qualified_name.to_lowercase())
                .or_default()
                .push(i);
            self.elements_by_kind
                .entry(elem.kind.clone())
                .or_default()
                .push(i);
        }

        for (i, rel) in self.relationships.iter().enumerate() {
            self.rels_by_source
                .entry(rel.source.clone())
                .or_default()
                .push(i);
            self.rels_by_target
                .entry(rel.target.clone())
                .or_default()
                .push(i);
        }
    }

    pub fn file_count(&self) -> usize {
        self.elements
            .iter()
            .map(|e| e.file_path.as_os_str())
            .collect::<HashSet<_>>()
            .len()
    }

    pub fn element_count(&self) -> usize {
        self.elements.len()
    }

    pub fn relationship_count(&self) -> usize {
        self.relationships.len()
    }

    #[cfg(feature = "vector")]
    pub fn build_vectors(&mut self) -> Result<(), String> {
        let vi = crate::vector::VectorIndex::build(&self.elements)?;
        self.vector_index = Some(vi);
        Ok(())
    }

    #[cfg(feature = "vector")]
    pub fn has_vectors(&self) -> bool {
        self.vector_index.is_some()
    }

    pub fn inspect(&self, name: &str) -> Option<serde_json::Value> {
        let elem = self.resolve_element(name)?;

        let rels_out: Vec<_> = self
            .relationships
            .iter()
            .filter(|r| Self::name_matches(&r.source, &elem.qualified_name))
            .map(|r| {
                serde_json::json!({
                    "kind": r.kind,
                    "target": r.target,
                    "file_path": r.file_path,
                })
            })
            .collect();

        let rels_in: Vec<_> = self
            .relationships
            .iter()
            .filter(|r| Self::name_matches(&r.target, &elem.qualified_name))
            .map(|r| {
                serde_json::json!({
                    "kind": r.kind,
                    "source": r.source,
                    "file_path": r.file_path,
                })
            })
            .collect();

        let layer = elem.layer.as_ref().map(|l| l.to_string());

        Some(serde_json::json!({
            "qualified_name": elem.qualified_name,
            "kind": elem.kind,
            "layer": layer,
            "file_path": elem.file_path,
            "span": {
                "start_line": elem.span.start_line,
                "start_col": elem.span.start_col,
                "end_line": elem.span.end_line,
                "end_col": elem.span.end_col,
            },
            "members": elem.members,
            "relationships_out": rels_out,
            "relationships_in": rels_in,
            "total_relationships": rels_out.len() + rels_in.len(),
        }))
    }

    pub fn save(&self, path: &Path) -> Result<(), CoreError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self, CoreError> {
        let json = std::fs::read_to_string(path)?;
        let mut graph: Self = serde_json::from_str(&json)?;
        graph.rebuild_indices();
        Ok(graph)
    }

    fn resolve_element(&self, name: &str) -> Option<&SysmlElement> {
        let lower = name.to_lowercase();
        if let Some(idxs) = self.elements_by_name.get(&lower) {
            return Some(&self.elements[idxs[0]]);
        }
        self.elements
            .iter()
            .find(|elem| elem.qualified_name.to_lowercase().contains(&lower))
    }

    fn name_matches(a: &str, b: &str) -> bool {
        let a_lower = a.to_lowercase();
        let b_lower = b.to_lowercase();
        if a_lower == b_lower {
            return true;
        }
        let a_short = a.rsplit("::").next().unwrap_or(a).to_lowercase();
        let b_short = b.rsplit("::").next().unwrap_or(b).to_lowercase();
        if a_short == b_lower || b_short == a_lower || a_short == b_short {
            return true;
        }
        if a_lower.ends_with(&format!("::{}", b_lower))
            || b_lower.ends_with(&format!("::{}", a_lower))
        {
            return true;
        }
        if a_lower.starts_with(&format!("{}::", b_lower))
            || b_lower.starts_with(&format!("{}::", a_lower))
        {
            return true;
        }
        if a_lower.contains(&format!("::{}", b_short))
            || b_lower.contains(&format!("::{}", a_short))
        {
            return true;
        }
        if a_lower.starts_with(&format!("{}.", b_short))
            || b_lower.starts_with(&format!("{}.", a_short))
        {
            return true;
        }
        if a_lower.contains('.') {
            let dotted_first = a_lower.split('.').next().unwrap_or("");
            if dotted_first == b_short || dotted_first == b_lower {
                return true;
            }
        }
        if b_lower.contains('.') {
            let dotted_first = b_lower.split('.').next().unwrap_or("");
            if dotted_first == a_short || dotted_first == a_lower {
                return true;
            }
        }
        false
    }

    fn find_edges(
        &self,
        element: &str,
        direction: &Direction,
        type_filter: &Option<Vec<String>>,
        include_structural: bool,
    ) -> Vec<(String, String, String, PathBuf)> {
        let mut edges = Vec::new();
        let structural_kinds: Vec<String> = STRUCTURAL_RELATIONSHIP_KINDS
            .iter()
            .map(|s| s.to_lowercase())
            .collect();

        for rel in &self.relationships {
            let rel_kind_lower = rel.kind.to_lowercase();
            if let Some(types) = type_filter {
                if !types.iter().any(|t| t.to_lowercase() == rel_kind_lower) {
                    continue;
                }
            } else if !include_structural && structural_kinds.iter().any(|s| s == &rel_kind_lower) {
                continue;
            }

            let is_source = Self::name_matches(&rel.source, element);
            let is_target = Self::name_matches(&rel.target, element);

            match direction {
                Direction::Forward => {
                    if is_source {
                        edges.push((
                            rel.source.clone(),
                            rel.kind.clone(),
                            rel.target.clone(),
                            rel.file_path.clone(),
                        ));
                    }
                }
                Direction::Backward => {
                    if is_target {
                        edges.push((
                            rel.source.clone(),
                            rel.kind.clone(),
                            rel.target.clone(),
                            rel.file_path.clone(),
                        ));
                    }
                }
                Direction::Both => {
                    if is_source || is_target {
                        edges.push((
                            rel.source.clone(),
                            rel.kind.clone(),
                            rel.target.clone(),
                            rel.file_path.clone(),
                        ));
                    }
                }
            }
        }
        edges
    }

    fn bfs_trace(&self, seed: &str, opts: &TraceOptions) -> Vec<TraceHop> {
        let max_hops = opts.max_hops.min(10);
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<(String, u32)> = VecDeque::new();
        let mut hops = Vec::new();

        let resolved = self
            .resolve_element(seed)
            .map(|e| e.qualified_name.clone())
            .unwrap_or_else(|| seed.to_string());

        visited.insert(resolved.to_lowercase());
        queue.push_back((resolved, 0));

        while let Some((current, depth)) = queue.pop_front() {
            if depth >= max_hops {
                continue;
            }

            let edges = self.find_edges(
                &current,
                &opts.direction,
                &opts.relationship_types,
                opts.include_structural,
            );

            for (source, rel_kind, target, file_path) in edges {
                let next = if Self::name_matches(&source, &current) {
                    target.clone()
                } else {
                    source.clone()
                };

                let source_elem = self.resolve_element(&source);
                let target_elem = self.resolve_element(&target);

                hops.push(TraceHop {
                    depth: depth + 1,
                    source: source.clone(),
                    relationship: rel_kind,
                    target: target.clone(),
                    file_path,
                    source_kind: source_elem.map(|e| e.kind.clone()),
                    target_kind: target_elem.map(|e| e.kind.clone()),
                    source_layer: source_elem.and_then(|e| e.layer.as_ref().map(|l| l.to_string())),
                    target_layer: target_elem.and_then(|e| e.layer.as_ref().map(|l| l.to_string())),
                });

                let next_lower = next.to_lowercase();
                if !visited.contains(&next_lower) {
                    visited.insert(next_lower);
                    queue.push_back((next, depth + 1));
                }
            }
        }

        hops
    }

    fn run_check(&self, check_type: &CheckType) -> Vec<Finding> {
        match check_type {
            CheckType::OrphanRequirements => self.check_orphan_requirements(),
            CheckType::UnverifiedRequirements => self.check_unverified_requirements(),
            CheckType::MissingVerification => self.check_missing_verification(),
            CheckType::UnconnectedPorts => self.check_unconnected_ports(),
            CheckType::DanglingReferences => self.check_dangling_references(),
        }
    }

    fn satisfy_targets(&self) -> HashSet<String> {
        self.relationships
            .iter()
            .filter(|r| r.kind.eq_ignore_ascii_case("satisfy"))
            .flat_map(|r| {
                let short = r
                    .target
                    .rsplit("::")
                    .next()
                    .unwrap_or(&r.target)
                    .to_string();
                vec![r.target.to_lowercase(), short.to_lowercase()]
            })
            .collect()
    }

    fn verify_targets(&self) -> HashSet<String> {
        self.relationships
            .iter()
            .filter(|r| r.kind.eq_ignore_ascii_case("verify"))
            .flat_map(|r| {
                let short = r
                    .target
                    .rsplit("::")
                    .next()
                    .unwrap_or(&r.target)
                    .to_string();
                vec![r.target.to_lowercase(), short.to_lowercase()]
            })
            .collect()
    }

    fn verify_sources(&self) -> HashSet<String> {
        self.relationships
            .iter()
            .filter(|r| r.kind.eq_ignore_ascii_case("verify"))
            .flat_map(|r| {
                let short = r
                    .source
                    .rsplit("::")
                    .next()
                    .unwrap_or(&r.source)
                    .to_string();
                vec![r.source.to_lowercase(), short.to_lowercase()]
            })
            .collect()
    }

    fn connect_endpoints(&self) -> HashSet<String> {
        self.relationships
            .iter()
            .filter(|r| r.kind.eq_ignore_ascii_case("connect"))
            .flat_map(|r| {
                let src_short = r
                    .source
                    .rsplit("::")
                    .next()
                    .unwrap_or(&r.source)
                    .to_string();
                let tgt_short = r
                    .target
                    .rsplit("::")
                    .next()
                    .unwrap_or(&r.target)
                    .to_string();
                vec![
                    r.source.to_lowercase(),
                    r.target.to_lowercase(),
                    src_short.to_lowercase(),
                    tgt_short.to_lowercase(),
                    format!("::{}", src_short.to_lowercase()),
                    format!("::{}", tgt_short.to_lowercase()),
                ]
            })
            .collect()
    }

    fn all_qualified_names(&self) -> HashSet<String> {
        self.elements
            .iter()
            .flat_map(|e| {
                let short = e
                    .qualified_name
                    .rsplit("::")
                    .next()
                    .unwrap_or(&e.qualified_name)
                    .to_string();
                vec![e.qualified_name.to_lowercase(), short.to_lowercase()]
            })
            .collect()
    }

    fn is_requirement(elem: &SysmlElement) -> bool {
        elem.kind.to_lowercase().contains("requirement")
    }

    fn is_port(elem: &SysmlElement) -> bool {
        elem.kind.to_lowercase().contains("port")
    }

    fn is_verification(elem: &SysmlElement) -> bool {
        let k = elem.kind.to_lowercase();
        k.contains("verification") || k.contains("verify")
    }

    fn check_orphan_requirements(&self) -> Vec<Finding> {
        let targets = self.satisfy_targets();
        self.elements
            .iter()
            .filter(|e| Self::is_requirement(e))
            .filter(|e| {
                let qname = e.qualified_name.to_lowercase();
                let short = e
                    .qualified_name
                    .rsplit("::")
                    .next()
                    .unwrap_or(&e.qualified_name)
                    .to_lowercase();
                !targets.contains(&qname) && !targets.contains(&short)
            })
            .map(|e| Finding {
                check_type: CheckType::OrphanRequirements,
                element: e.qualified_name.clone(),
                message: "Requirement has no satisfy relationship".to_string(),
                file_path: e.file_path.clone(),
                span: e.span.clone(),
            })
            .collect()
    }

    fn check_unverified_requirements(&self) -> Vec<Finding> {
        let targets = self.verify_targets();
        self.elements
            .iter()
            .filter(|e| Self::is_requirement(e))
            .filter(|e| {
                let qname = e.qualified_name.to_lowercase();
                let short = e
                    .qualified_name
                    .rsplit("::")
                    .next()
                    .unwrap_or(&e.qualified_name)
                    .to_lowercase();
                !targets.contains(&qname) && !targets.contains(&short)
            })
            .map(|e| Finding {
                check_type: CheckType::UnverifiedRequirements,
                element: e.qualified_name.clone(),
                message: "Requirement has no verify relationship".to_string(),
                file_path: e.file_path.clone(),
                span: e.span.clone(),
            })
            .collect()
    }

    fn check_missing_verification(&self) -> Vec<Finding> {
        let sources = self.verify_sources();
        self.elements
            .iter()
            .filter(|e| Self::is_verification(e))
            .filter(|e| {
                let qname = e.qualified_name.to_lowercase();
                let short = e
                    .qualified_name
                    .rsplit("::")
                    .next()
                    .unwrap_or(&e.qualified_name)
                    .to_lowercase();
                !sources.contains(&qname) && !sources.contains(&short)
            })
            .map(|e| Finding {
                check_type: CheckType::MissingVerification,
                element: e.qualified_name.clone(),
                message: "Verification element is not a source of any verify relationship"
                    .to_string(),
                file_path: e.file_path.clone(),
                span: e.span.clone(),
            })
            .collect()
    }

    fn check_unconnected_ports(&self) -> Vec<Finding> {
        let endpoints = self.connect_endpoints();
        self.elements
            .iter()
            .filter(|e| Self::is_port(e))
            .filter(|e| {
                let qname = e.qualified_name.to_lowercase();
                let short = e
                    .qualified_name
                    .rsplit("::")
                    .next()
                    .unwrap_or(&e.qualified_name)
                    .to_lowercase();
                !endpoints.contains(&qname)
                    && !endpoints.contains(&short)
                    && !endpoints.contains(&format!("::{}", short))
            })
            .map(|e| Finding {
                check_type: CheckType::UnconnectedPorts,
                element: e.qualified_name.clone(),
                message: "Port has no connection".to_string(),
                file_path: e.file_path.clone(),
                span: e.span.clone(),
            })
            .collect()
    }

    fn check_dangling_references(&self) -> Vec<Finding> {
        let names = self.all_qualified_names();
        self.relationships
            .iter()
            .filter(|r| {
                let k = r.kind.to_lowercase();
                !STRUCTURAL_RELATIONSHIP_KINDS
                    .iter()
                    .any(|s| s.eq_ignore_ascii_case(&k))
            })
            .filter(|r| {
                let target_lower = r.target.to_lowercase();
                let target_short = r
                    .target
                    .rsplit("::")
                    .next()
                    .unwrap_or(&r.target)
                    .to_lowercase();
                !names.contains(&target_lower) && !names.contains(&target_short)
            })
            .map(|r| Finding {
                check_type: CheckType::DanglingReferences,
                element: r.source.clone(),
                message: format!("Relationship target '{}' not found in index", r.target),
                file_path: r.file_path.clone(),
                span: r.span.clone(),
            })
            .collect()
    }

    fn score_elements(&self, query: &str) -> Vec<(usize, f64)> {
        let eq = expand_query(query);
        self.score_elements_expanded(&eq)
    }

    fn score_elements_expanded(&self, eq: &ExpandedQuery) -> Vec<(usize, f64)> {
        const W_EXACT_NAME: f64 = 2.0;
        const W_PARTIAL_NAME: f64 = 1.0;
        const W_KIND_MATCH: f64 = 0.7;
        const W_REL_MATCH: f64 = 0.8;
        const W_DOC_MATCH: f64 = 0.4;
        #[cfg(feature = "vector")]
        const W_SEMANTIC: f64 = 0.9;
        const W_1HOP: f64 = 0.6;
        const W_2HOP: f64 = 0.3;
        const W_IMPORT_ADJ: f64 = 0.2;

        let mut element_scores: Vec<f64> = vec![0.0; self.elements.len()];
        let mut rel_scores: Vec<f64> = vec![0.0; self.elements.len()];
        const REL_SCORE_CAP: f64 = 1.5;

        for (i, elem) in self.elements.iter().enumerate() {
            let name_lower = elem.qualified_name.to_lowercase();
            let short_name = elem
                .qualified_name
                .rsplit("::")
                .next()
                .unwrap_or(&elem.qualified_name)
                .to_lowercase();

            for token in &eq.tokens {
                if name_lower == *token || short_name == *token {
                    element_scores[i] += W_EXACT_NAME;
                } else if token.len() >= 4
                    && (name_lower.contains(token.as_str()) || short_name.contains(token.as_str()))
                {
                    element_scores[i] += W_PARTIAL_NAME;
                }
            }

            for kind in &eq.element_kinds {
                if elem.kind == *kind {
                    element_scores[i] += W_KIND_MATCH;
                }
            }

            if let Some(doc) = &elem.doc {
                let doc_lower = doc.to_lowercase();
                for token in &eq.tokens {
                    if doc_lower.contains(token.as_str()) {
                        element_scores[i] += W_DOC_MATCH;
                    }
                }
            }
        }

        for rel in &self.relationships {
            let rel_kind_lower = rel.kind.to_lowercase();
            for rk in &eq.relationship_kinds {
                if rel_kind_lower == rk.to_lowercase() {
                    if let Some(src_idx) = self.find_element_index(&rel.source) {
                        rel_scores[src_idx] += W_REL_MATCH;
                    }
                    if let Some(tgt_idx) = self.find_element_index(&rel.target) {
                        rel_scores[tgt_idx] += W_REL_MATCH;
                    }
                }
            }

            for token in &eq.tokens {
                if token.len() < 4 {
                    continue;
                }
                let src_lower = rel.source.to_lowercase();
                let tgt_lower = rel.target.to_lowercase();
                if src_lower.contains(token.as_str()) || tgt_lower.contains(token.as_str()) {
                    if let Some(src_idx) = self.find_element_index(&rel.source) {
                        rel_scores[src_idx] += W_REL_MATCH * 0.5;
                    }
                    if let Some(tgt_idx) = self.find_element_index(&rel.target) {
                        rel_scores[tgt_idx] += W_REL_MATCH * 0.5;
                    }
                }
            }
        }

        for i in 0..self.elements.len() {
            element_scores[i] += rel_scores[i].min(REL_SCORE_CAP);
        }

        #[cfg(feature = "vector")]
        if let Some(ref vi) = self.vector_index {
            let raw_query = eq.tokens.join(" ");
            if let Ok(sims) = vi.element_similarities(&raw_query) {
                for (i, elem) in self.elements.iter().enumerate() {
                    if let Some(&sim) = sims.get(&elem.qualified_name) {
                        element_scores[i] += W_SEMANTIC * sim;
                    }
                }
            }
        }

        let matched_indices: HashSet<usize> = element_scores
            .iter()
            .enumerate()
            .filter(|(_, &s)| s > 0.0)
            .map(|(i, _)| i)
            .collect();

        let matched_names: HashSet<&str> = matched_indices
            .iter()
            .map(|&i| self.elements[i].qualified_name.as_str())
            .collect();

        let neighbors_1hop = self.find_element_neighbors(&matched_names, 1);
        let neighbors_2hop_raw = self.find_element_neighbors(&matched_names, 2);
        let neighbors_2hop: HashSet<usize> = neighbors_2hop_raw
            .difference(&neighbors_1hop)
            .copied()
            .collect();
        let import_adjacent = self.find_import_adjacent_elements(&matched_names);

        for &idx in &neighbors_1hop {
            if matched_indices.contains(&idx) {
                element_scores[idx] *= 1.0 + W_1HOP;
            }
        }
        for &idx in &neighbors_2hop {
            if matched_indices.contains(&idx) {
                element_scores[idx] *= 1.0 + W_2HOP;
            }
        }
        for &idx in &import_adjacent {
            if matched_indices.contains(&idx) {
                element_scores[idx] *= 1.0 + W_IMPORT_ADJ;
            }
        }

        let max_score = element_scores.iter().copied().fold(0.0_f64, f64::max);
        if max_score > 0.0 {
            for s in &mut element_scores {
                if *s > 0.0 {
                    *s /= max_score;
                }
            }
        }

        element_scores
            .into_iter()
            .enumerate()
            .filter(|(_, s)| *s > 0.0)
            .collect()
    }

    fn find_element_index(&self, qualified_name: &str) -> Option<usize> {
        let lower = qualified_name.to_lowercase();
        if let Some(idxs) = self.elements_by_name.get(&lower) {
            return Some(idxs[0]);
        }
        None
    }

    fn find_element_neighbors(&self, seed_names: &HashSet<&str>, hops: usize) -> HashSet<usize> {
        let mut current_names: HashSet<String> = seed_names.iter().map(|s| s.to_string()).collect();
        let mut all_neighbors: HashSet<usize> = HashSet::new();

        for _ in 0..hops {
            let mut next_hop_names: HashSet<String> = HashSet::new();
            for rel in &self.relationships {
                let src_match = current_names.contains(&rel.source);
                let tgt_match = current_names.contains(&rel.target);
                if src_match && !seed_names.contains(rel.target.as_str()) {
                    next_hop_names.insert(rel.target.clone());
                }
                if tgt_match && !seed_names.contains(rel.source.as_str()) {
                    next_hop_names.insert(rel.source.clone());
                }
            }
            for name in &next_hop_names {
                let lower = name.to_lowercase();
                if let Some(idxs) = self.elements_by_name.get(&lower) {
                    all_neighbors.extend(idxs);
                }
            }
            current_names = next_hop_names;
        }

        all_neighbors
    }

    fn find_import_adjacent_elements(&self, matched_names: &HashSet<&str>) -> HashSet<usize> {
        let mut adjacent = HashSet::new();
        let matched_files: HashSet<&PathBuf> = matched_names
            .iter()
            .filter_map(|name| {
                let lower = name.to_lowercase();
                self.elements_by_name
                    .get(&lower)
                    .and_then(|idxs| idxs.first())
                    .map(|&i| &self.elements[i].file_path)
            })
            .collect();

        for rel in &self.relationships {
            if rel.kind.eq_ignore_ascii_case("import") && matched_files.contains(&rel.file_path) {
                let target_ns = rel.target.split("::").next().unwrap_or("");
                let target_lower = target_ns.to_lowercase();
                for (i, elem) in self.elements.iter().enumerate() {
                    let short = elem
                        .qualified_name
                        .rsplit("::")
                        .next()
                        .unwrap_or(&elem.qualified_name);
                    if elem.qualified_name.to_lowercase() == target_lower
                        || short.to_lowercase() == target_lower
                    {
                        adjacent.insert(i);
                    }
                }
            }
        }
        adjacent
    }

    fn build_search_result(
        &self,
        elem: &SysmlElement,
        score: f64,
        level: &DetailLevel,
    ) -> SearchResult {
        let rel_count = self
            .rels_by_source
            .get(&elem.qualified_name)
            .map(|v| v.len())
            .unwrap_or(0)
            + self
                .rels_by_target
                .get(&elem.qualified_name)
                .map(|v| v.len())
                .unwrap_or(0);

        let detail = match level {
            DetailLevel::L0 => serde_json::json!({}),
            DetailLevel::L1 => serde_json::json!({
                "start_line": elem.span.start_line,
                "end_line": elem.span.end_line,
                "file_path": elem.file_path,
                "relationship_count": rel_count,
                "layer": elem.layer.as_ref().map(|l| l.to_string()),
            }),
            DetailLevel::L2 => {
                let rels_out: Vec<serde_json::Value> = self
                    .rels_by_source
                    .get(&elem.qualified_name)
                    .map(|idxs| {
                        idxs.iter()
                            .map(|&ri| {
                                let r = &self.relationships[ri];
                                serde_json::json!({
                                    "kind": r.kind,
                                    "target": r.target,
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let rels_in: Vec<serde_json::Value> = self
                    .rels_by_target
                    .get(&elem.qualified_name)
                    .map(|idxs| {
                        idxs.iter()
                            .map(|&ri| {
                                let r = &self.relationships[ri];
                                serde_json::json!({
                                    "kind": r.kind,
                                    "source": r.source,
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                serde_json::json!({
                    "start_line": elem.span.start_line,
                    "end_line": elem.span.end_line,
                    "file_path": elem.file_path,
                    "relationship_count": rel_count,
                    "layer": elem.layer.as_ref().map(|l| l.to_string()),
                    "relationships_out": rels_out,
                    "relationships_in": rels_in,
                    "doc": elem.doc,
                })
            }
        };

        SearchResult {
            qualified_name: elem.qualified_name.clone(),
            kind: elem.kind.clone(),
            file_path: elem.file_path.clone(),
            span: elem.span.clone(),
            score,
            detail,
        }
    }
}

impl KnowledgeGraph for SysmlGraph {
    type Elem = SysmlElement;
    type Rel = SysmlRelationship;

    fn index(
        &mut self,
        results: Vec<ParseResult<Self::Elem, Self::Rel>>,
    ) -> Result<(), IndexError> {
        self.elements.clear();
        self.relationships.clear();

        for result in results {
            self.elements.extend(result.elements);
            self.relationships.extend(result.relationships);
        }

        crate::resolve::resolve_imports(&self.elements, &mut self.relationships);

        self.rebuild_indices();
        Ok(())
    }

    fn search(&self, query: &str, level: DetailLevel, limit: usize) -> Vec<SearchResult> {
        let mut scored = self.score_elements(query);
        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    self.elements[a.0]
                        .qualified_name
                        .cmp(&self.elements[b.0].qualified_name)
                })
        });

        scored
            .into_iter()
            .take(limit)
            .map(|(i, score)| self.build_search_result(&self.elements[i], score, &level))
            .collect()
    }

    fn trace(&self, element: &str, opts: TraceOptions) -> TraceResult {
        let root = self
            .resolve_element(element)
            .map(|e| e.qualified_name.clone())
            .unwrap_or_else(|| element.to_string());

        let mut hops = self.bfs_trace(element, &opts);

        if opts.format == TraceFormat::Flat {
            let mut seen = HashSet::new();
            hops.retain(|h| {
                let key = format!(
                    "{}|{}|{}",
                    h.source.to_lowercase(),
                    h.relationship.to_lowercase(),
                    h.target.to_lowercase()
                );
                seen.insert(key)
            });
        }

        TraceResult {
            root,
            hops,
            format: opts.format,
        }
    }

    fn check(&self, check_type: CheckType) -> Vec<Finding> {
        self.run_check(&check_type)
    }

    fn query(&self, predicate: Predicate) -> Vec<Triple> {
        self.relationships
            .iter()
            .filter(|r| {
                let r_kind = r.kind.to_lowercase();
                if let Some(ref rk) = predicate.relationship_kind {
                    if !r_kind.contains(&rk.to_lowercase()) {
                        return false;
                    }
                }
                if let Some(ref erk) = predicate.exclude_relationship_kind {
                    for exclude in erk.to_lowercase().split(',') {
                        let exclude = exclude.trim();
                        if !exclude.is_empty() && r_kind.contains(exclude) {
                            return false;
                        }
                    }
                }
                if let Some(ref sn) = predicate.source_name {
                    if !r.source.to_lowercase().contains(&sn.to_lowercase()) {
                        return false;
                    }
                }
                if let Some(ref tn) = predicate.target_name {
                    if !r.target.to_lowercase().contains(&tn.to_lowercase()) {
                        return false;
                    }
                }
                if let Some(ref sk) = predicate.source_kind {
                    let resolved = self.elements.iter().find(|e| e.qualified_name == r.source);
                    match resolved {
                        Some(e) => {
                            if !e.kind.to_lowercase().contains(&sk.to_lowercase()) {
                                return false;
                            }
                        }
                        None => return false,
                    }
                }
                if let Some(ref tk) = predicate.target_kind {
                    let resolved = self.elements.iter().find(|e| e.qualified_name == r.target);
                    match resolved {
                        Some(e) => {
                            if !e.kind.to_lowercase().contains(&tk.to_lowercase()) {
                                return false;
                            }
                        }
                        None => return false,
                    }
                }
                true
            })
            .map(|r| Triple {
                source: r.source.clone(),
                relationship: r.kind.clone(),
                target: r.target.clone(),
                file_path: r.file_path.clone(),
            })
            .collect()
    }

    fn elements(&self) -> &[Self::Elem] {
        &self.elements
    }

    fn relationships(&self) -> &[Self::Rel] {
        &self.relationships
    }
}

pub fn find_index(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let candidate = current.join(".nomograph").join("index.json");
        if candidate.exists() {
            return Some(candidate);
        }
        if !current.pop() {
            return None;
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use nomograph_core::traits::Parser as NomographParser;
    use std::path::Path;

    fn fixture_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/eve")
    }

    pub(crate) fn parse_all_eve() -> Vec<ParseResult<SysmlElement, SysmlRelationship>> {
        let parser = crate::parser::SysmlParser::new();
        let mut results = Vec::new();
        for entry in walkdir(fixture_dir()) {
            if entry.extension().and_then(|e| e.to_str()) == Some("sysml") {
                let source = std::fs::read_to_string(&entry).expect("read fixture");
                let result = parser.parse(&source, &entry).expect("parse fixture");
                results.push(result);
            }
        }
        results
    }

    fn walkdir(dir: PathBuf) -> Vec<PathBuf> {
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    files.extend(walkdir(path));
                } else {
                    files.push(path);
                }
            }
        }
        files
    }

    #[test]
    fn test_index_eve_model() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).expect("index should succeed");
        assert!(graph.element_count() > 0, "should have elements");
        assert!(graph.relationship_count() > 0, "should have relationships");
        assert_eq!(graph.file_count(), 19);
    }

    #[test]
    fn test_search_by_name() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let hits = graph.search("ShieldModule", DetailLevel::L1, 10);
        assert!(
            !hits.is_empty(),
            "ShieldModule should appear in search results"
        );
        assert!(
            hits.iter()
                .any(|h| h.qualified_name.contains("ShieldModule")),
            "at least one result should contain ShieldModule"
        );
    }

    #[test]
    fn test_search_by_kind() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let hits = graph.search("requirement_definition", DetailLevel::L0, 50);
        assert!(!hits.is_empty(), "should find requirement elements");
        let req_hits: Vec<_> = hits
            .iter()
            .filter(|h| h.kind.contains("requirement"))
            .collect();
        assert!(
            !req_hits.is_empty(),
            "should have requirement elements in results"
        );
        assert!(
            hits[0].kind.contains("requirement"),
            "top result should be a requirement element, got {}",
            hits[0].kind
        );
    }

    #[test]
    fn test_search_exact_name() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let hits = graph.search(
            "oreExtractionEfficiencyRequirementLowSec",
            DetailLevel::L1,
            10,
        );
        assert!(
            !hits.is_empty(),
            "oreExtractionEfficiencyRequirementLowSec should appear in search results"
        );
        assert!(hits[0]
            .qualified_name
            .contains("oreExtractionEfficiencyRequirementLowSec"));
        assert_eq!(hits[0].score, 1.0);
    }

    #[test]
    fn test_search_kind_expansion() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let hits = graph.search("requirement", DetailLevel::L0, 50);
        assert!(!hits.is_empty(), "should find elements for 'requirement'");
        let req_kind_hits: Vec<_> = hits
            .iter()
            .filter(|h| h.kind.contains("requirement"))
            .collect();
        assert!(
            !req_kind_hits.is_empty(),
            "should have requirement_definition or requirement_usage elements via vocabulary expansion"
        );
        let top_5_has_req = hits.iter().take(5).any(|h| h.kind.contains("requirement"));
        assert!(
            top_5_has_req,
            "top 5 results should include at least one requirement element"
        );
    }

    #[test]
    fn test_trace_from_element() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let result = graph.trace(
            "ShieldModule",
            TraceOptions {
                direction: Direction::Both,
                max_hops: 2,
                relationship_types: None,
                format: TraceFormat::Chain,
                include_structural: false,
            },
        );
        assert!(!result.hops.is_empty(), "trace should find connected hops");
        assert!(
            result.hops.iter().any(|h| h.relationship == "TypedBy"),
            "should find TypedBy relationships"
        );
    }

    #[test]
    fn test_trace_with_type_filter() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let result = graph.trace(
            "ShieldModule",
            TraceOptions {
                direction: Direction::Both,
                max_hops: 3,
                relationship_types: Some(vec!["TypedBy".to_string()]),
                format: TraceFormat::Chain,
                include_structural: false,
            },
        );
        for hop in &result.hops {
            assert_eq!(
                hop.relationship.to_lowercase(),
                "typedby",
                "all hops should be TypedBy when filtered"
            );
        }
    }

    #[test]
    fn test_trace_flat_deduplicates() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let chain_result = graph.trace(
            "ShieldModule",
            TraceOptions {
                direction: Direction::Both,
                max_hops: 3,
                relationship_types: None,
                format: TraceFormat::Chain,
                include_structural: false,
            },
        );
        let flat_result = graph.trace(
            "ShieldModule",
            TraceOptions {
                direction: Direction::Both,
                max_hops: 3,
                relationship_types: None,
                format: TraceFormat::Flat,
                include_structural: false,
            },
        );
        assert!(
            flat_result.hops.len() <= chain_result.hops.len(),
            "flat format should have fewer or equal hops than chain"
        );
    }

    #[test]
    fn test_check_orphan_requirements() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let findings = graph.check(CheckType::OrphanRequirements);
        for f in &findings {
            assert_eq!(f.check_type, CheckType::OrphanRequirements);
        }
    }

    #[test]
    fn test_check_unconnected_ports() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let findings = graph.check(CheckType::UnconnectedPorts);
        for f in &findings {
            assert_eq!(f.check_type, CheckType::UnconnectedPorts);
        }
    }

    #[test]
    fn test_check_dangling_references() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let findings = graph.check(CheckType::DanglingReferences);
        for f in &findings {
            assert_eq!(f.check_type, CheckType::DanglingReferences);
            assert!(!f.message.is_empty(), "finding should have a message");
        }
    }

    #[test]
    fn test_query_satisfy() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let triples = graph.query(Predicate {
            source_kind: None,
            source_name: None,
            relationship_kind: Some("satisfy".to_string()),
            target_kind: None,
            target_name: None,
            exclude_relationship_kind: None,
        });
        for t in &triples {
            assert!(
                t.relationship.to_lowercase().contains("satisfy"),
                "all results should be satisfy relationships"
            );
        }
    }

    #[test]
    fn test_query_typed_by() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let triples = graph.query(Predicate {
            source_kind: None,
            source_name: None,
            relationship_kind: Some("typedby".to_string()),
            target_kind: None,
            target_name: None,
            exclude_relationship_kind: None,
        });
        assert!(
            !triples.is_empty(),
            "Eve model should have TypedBy relationships"
        );
    }

    #[test]
    fn test_query_with_name_filter() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let triples = graph.query(Predicate {
            source_kind: None,
            source_name: Some("shield".to_string()),
            relationship_kind: None,
            target_kind: None,
            target_name: None,
            exclude_relationship_kind: None,
        });
        for t in &triples {
            assert!(
                t.source.to_lowercase().contains("shield"),
                "source should contain 'shield'"
            );
        }
    }

    #[test]
    fn test_import_resolution_qualifies_targets() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let typed_by: Vec<_> = graph
            .relationships()
            .iter()
            .filter(|r| {
                r.kind == "TypedBy"
                    && r.target
                        == "MiningFrigateRequirementsDef::OreExtractionEfficiencyRequirement"
            })
            .collect();
        assert!(
            !typed_by.is_empty(),
            "import resolution should qualify OreExtractionEfficiencyRequirement"
        );
    }

    #[test]
    fn test_members_populated() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let has_members = graph.elements().iter().any(|e| !e.members.is_empty());
        assert!(has_members, "at least one element should have members");
        let pkg = graph
            .elements()
            .iter()
            .find(|e| e.kind == "package_definition" && !e.members.is_empty());
        assert!(pkg.is_some(), "package elements should have member lists");
    }

    #[test]
    fn test_cross_file_trace() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let result = graph.trace(
            "OreExtractionEfficiencyRequirement",
            TraceOptions {
                direction: Direction::Both,
                max_hops: 1,
                relationship_types: None,
                format: TraceFormat::Flat,
                include_structural: false,
            },
        );
        assert!(
            !result.hops.is_empty(),
            "cross-file trace should find relationships"
        );
        let files: HashSet<_> = result.hops.iter().map(|h| h.file_path.clone()).collect();
        assert!(files.len() >= 1, "trace should span at least one file");
    }

    #[test]
    fn test_name_matches_dotted_feature_chain() {
        assert!(SysmlGraph::name_matches("shield.port", "shield"));
        assert!(SysmlGraph::name_matches("shield", "shield.port"));
        assert!(SysmlGraph::name_matches("Pkg::shield", "shield.port"));
        assert!(SysmlGraph::name_matches("shield::subpart", "shield"));
        assert!(!SysmlGraph::name_matches("shield", "armor"));
    }

    #[test]
    fn test_save_load_roundtrip() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();

        let tmp = std::env::temp_dir().join("nomograph_test_index.json");
        graph.save(&tmp).expect("save should succeed");

        let loaded = SysmlGraph::load(&tmp).expect("load should succeed");
        assert_eq!(loaded.element_count(), graph.element_count());
        assert_eq!(loaded.relationship_count(), graph.relationship_count());

        let hits_orig = graph.search("MFRQ01", DetailLevel::L1, 5);
        let hits_loaded = loaded.search("MFRQ01", DetailLevel::L1, 5);
        assert_eq!(hits_orig.len(), hits_loaded.len());
        if !hits_orig.is_empty() {
            assert_eq!(hits_orig[0].qualified_name, hits_loaded[0].qualified_name);
        }

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_search_multi_word_query() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let hits = graph.search("shield module", DetailLevel::L1, 20);
        assert!(
            !hits.is_empty(),
            "multi-word query 'shield module' should return results"
        );
        assert!(
            hits.iter()
                .any(|h| h.qualified_name.to_lowercase().contains("shield")),
            "results should include elements containing 'shield'"
        );
    }

    #[test]
    fn test_search_scores_normalized() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let hits = graph.search("ore extraction requirement", DetailLevel::L0, 50);
        assert!(!hits.is_empty());
        assert!(
            (hits[0].score - 1.0).abs() < 1e-9,
            "top score should be 1.0 after normalization, got {}",
            hits[0].score
        );
        for hit in &hits {
            assert!(
                hit.score > 0.0 && hit.score <= 1.0,
                "score {} out of range",
                hit.score
            );
        }
    }

    #[test]
    fn test_search_empty_query() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let hits = graph.search("", DetailLevel::L0, 10);
        assert!(hits.is_empty(), "empty query should return no results");
    }

    #[test]
    fn test_rflp_layer_populated() {
        use crate::element::RflpLayer;
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();

        let elements = graph.elements();
        let with_layer = elements.iter().filter(|e| e.layer.is_some()).count();
        assert!(
            with_layer > 0,
            "at least some elements should have RFLP layer"
        );

        let req_elements: Vec<_> = elements
            .iter()
            .filter(|e| e.kind.contains("requirement"))
            .collect();
        assert!(!req_elements.is_empty());
        for elem in &req_elements {
            assert_eq!(
                elem.layer,
                Some(RflpLayer::Requirements),
                "{} should be Requirements layer",
                elem.qualified_name
            );
        }

        let part_elements: Vec<_> = elements
            .iter()
            .filter(|e| e.kind.contains("part"))
            .collect();
        assert!(!part_elements.is_empty());
        for elem in &part_elements {
            assert_eq!(
                elem.layer,
                Some(RflpLayer::Logical),
                "{} should be Logical layer",
                elem.qualified_name
            );
        }
    }

    #[test]
    fn test_rflp_layer_in_search_detail() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();

        let hits = graph.search("requirement", DetailLevel::L1, 5);
        assert!(!hits.is_empty());
        for hit in &hits {
            if hit.kind.contains("requirement") {
                let layer = hit.detail.get("layer").and_then(|v| v.as_str());
                assert_eq!(
                    layer,
                    Some("R"),
                    "{} should show R layer in detail",
                    hit.qualified_name
                );
            }
        }
    }

    #[test]
    fn test_rflp_layer_roundtrip() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();

        let dir = std::env::temp_dir().join("nomograph_rflp_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("index.json");
        graph.save(&path).unwrap();

        let loaded = SysmlGraph::load(&path).unwrap();
        let original_layers: Vec<_> = graph.elements().iter().map(|e| e.layer).collect();
        let loaded_layers: Vec<_> = loaded.elements().iter().map(|e| e.layer).collect();
        assert_eq!(
            original_layers, loaded_layers,
            "RFLP layers should survive serialization roundtrip"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_query_returns_member_relationships() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let triples = graph.query(Predicate {
            source_kind: None,
            source_name: None,
            relationship_kind: Some("member".to_string()),
            target_kind: None,
            target_name: None,
            exclude_relationship_kind: None,
        });
        assert!(
            !triples.is_empty(),
            "query --rel member should return results (was previously filtered)"
        );
        for t in &triples {
            assert_eq!(t.relationship, "Member");
        }
    }

    #[test]
    fn test_query_returns_import_relationships() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let triples = graph.query(Predicate {
            source_kind: None,
            source_name: None,
            relationship_kind: Some("import".to_string()),
            target_kind: None,
            target_name: None,
            exclude_relationship_kind: None,
        });
        assert!(
            !triples.is_empty(),
            "query --rel import should return results (was previously filtered)"
        );
        for t in &triples {
            assert_eq!(t.relationship, "Import");
        }
    }

    #[test]
    fn test_query_unfiltered_includes_all_types() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let triples = graph.query(Predicate {
            source_kind: None,
            source_name: None,
            relationship_kind: None,
            target_kind: None,
            target_name: None,
            exclude_relationship_kind: None,
        });
        let kinds: HashSet<_> = triples.iter().map(|t| t.relationship.as_str()).collect();
        assert!(
            kinds.contains("Member"),
            "unfiltered query should include Member"
        );
        assert!(
            kinds.contains("Import"),
            "unfiltered query should include Import"
        );
        assert!(
            kinds.contains("TypedBy"),
            "unfiltered query should include TypedBy"
        );
    }

    #[test]
    fn test_trace_with_include_structural() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let without = graph.trace(
            "MiningFrigateRequirementsDef",
            TraceOptions {
                direction: Direction::Both,
                max_hops: 1,
                relationship_types: None,
                format: TraceFormat::Chain,
                include_structural: false,
            },
        );
        let with = graph.trace(
            "MiningFrigateRequirementsDef",
            TraceOptions {
                direction: Direction::Both,
                max_hops: 1,
                relationship_types: None,
                format: TraceFormat::Chain,
                include_structural: true,
            },
        );
        assert!(
            with.hops.len() > without.hops.len(),
            "include_structural should return more hops ({} vs {})",
            with.hops.len(),
            without.hops.len()
        );
        let has_member = with.hops.iter().any(|h| h.relationship == "Member");
        assert!(has_member, "structural trace should include Member hops");
    }

    #[test]
    fn test_trace_explicit_member_type_filter() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let result = graph.trace(
            "MiningFrigateRequirementsDef",
            TraceOptions {
                direction: Direction::Forward,
                max_hops: 1,
                relationship_types: Some(vec!["Member".to_string()]),
                format: TraceFormat::Chain,
                include_structural: false,
            },
        );
        assert!(
            !result.hops.is_empty(),
            "explicit --types Member should return Member hops even without include_structural"
        );
        for hop in &result.hops {
            assert_eq!(hop.relationship, "Member");
        }
    }

    #[test]
    fn test_query_exclude_rel() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let triples = graph.query(Predicate {
            source_kind: None,
            source_name: Some("MiningFrigateRequirementsDef".to_string()),
            relationship_kind: None,
            target_kind: None,
            target_name: None,
            exclude_relationship_kind: Some("member,import".to_string()),
        });
        for t in &triples {
            assert_ne!(
                t.relationship, "Member",
                "excluded Member should not appear"
            );
            assert_ne!(
                t.relationship, "Import",
                "excluded Import should not appear"
            );
        }
        assert!(
            !triples.is_empty(),
            "should still have non-excluded relationships"
        );
    }

    #[test]
    fn test_inspect_element() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let result = graph.inspect("MiningFrigateRequirementsDef");
        assert!(result.is_some(), "inspect should find the element");
        let val = result.unwrap();
        assert_eq!(val["kind"], "package_definition");
        assert!(!val["members"].as_array().unwrap().is_empty());
        assert!(!val["relationships_out"].as_array().unwrap().is_empty());
        assert!(!val["relationships_in"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_inspect_not_found() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        assert!(graph.inspect("NonExistentElement12345").is_none());
    }

    #[test]
    fn test_trace_hops_have_element_metadata() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let result = graph.trace(
            "ShieldModule",
            TraceOptions {
                direction: Direction::Both,
                max_hops: 1,
                relationship_types: None,
                format: TraceFormat::Chain,
                include_structural: false,
            },
        );
        assert!(!result.hops.is_empty());
        let has_kind = result.hops.iter().any(|h| h.source_kind.is_some());
        assert!(has_kind, "trace hops should include source_kind metadata");
    }

    #[test]
    fn test_message_relationships_extracted() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let messages: Vec<_> = graph
            .relationships
            .iter()
            .filter(|r| r.kind == "Message")
            .collect();
        assert!(
            messages.len() >= 20,
            "expected >=20 Message relationships from Domain.sysml, got {}",
            messages.len()
        );
        let has_from_to = messages
            .iter()
            .any(|r| r.source.contains("controlPort") || r.target.contains("controlPort"));
        assert!(has_from_to, "Message should extract from/to port paths");
    }

    #[test]
    fn test_flow_usage_extracted() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let flows: Vec<_> = graph
            .relationships
            .iter()
            .filter(|r| r.kind == "Flow")
            .collect();
        assert!(
            flows.len() >= 10,
            "expected >=10 Flow relationships (including flow_usage), got {}",
            flows.len()
        );
    }

    #[test]
    fn test_first_statement_extracted() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let successions: Vec<_> = graph
            .relationships
            .iter()
            .filter(|r| r.kind == "Succession")
            .collect();
        assert!(
            successions.len() >= 10,
            "expected >=10 Succession relationships (including first_statement), got {}",
            successions.len()
        );
    }

    #[test]
    fn test_redefines_statement_extracted() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let redefines: Vec<_> = graph
            .relationships
            .iter()
            .filter(|r| r.kind == "Redefine")
            .collect();
        assert!(
            redefines.len() >= 12,
            "expected >=12 Redefine relationships (including redefines_statement), got {}",
            redefines.len()
        );
    }

    #[test]
    fn test_exhibit_usage_extracted() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let exhibits: Vec<_> = graph
            .relationships
            .iter()
            .filter(|r| r.kind == "Exhibit")
            .collect();
        assert!(
            !exhibits.is_empty(),
            "expected at least 1 Exhibit relationship from exhibit_usage"
        );
    }
}

#[cfg(test)]
mod coverage_tests {
    use std::collections::HashSet;

    use super::tests::parse_all_eve;
    use super::*;
    use crate::vocabulary::{
        classify_layer, ELEMENT_KIND_NAMES, RELATIONSHIP_KIND_NAMES, STRUCTURAL_RELATIONSHIP_KINDS,
    };
    use crate::walker::{RelationshipKind, RELATIONSHIP_DISPATCH};

    #[test]
    fn test_relationship_kind_names_covers_all_variants() {
        let names: HashSet<&str> = RELATIONSHIP_KIND_NAMES.iter().copied().collect();
        for variant in RelationshipKind::ALL {
            let display = variant.to_string();
            assert!(
                names.contains(display.as_str()),
                "RelationshipKind::{display} missing from RELATIONSHIP_KIND_NAMES in vocabulary.rs"
            );
        }
    }

    #[test]
    fn test_relationship_kind_names_no_stale_entries() {
        let variant_names: HashSet<String> = RelationshipKind::ALL
            .iter()
            .map(|v| v.to_string())
            .collect();
        for name in RELATIONSHIP_KIND_NAMES {
            assert!(
                variant_names.contains(*name),
                "RELATIONSHIP_KIND_NAMES contains '{name}' but no RelationshipKind variant produces it"
            );
        }
    }

    #[test]
    fn test_dispatch_covers_all_non_synthetic_variants() {
        let dispatched: HashSet<RelationshipKind> =
            RELATIONSHIP_DISPATCH.iter().map(|(_, v)| *v).collect();
        let synthetic = [RelationshipKind::TypedBy, RelationshipKind::Member];
        for variant in RelationshipKind::ALL {
            if synthetic.contains(variant) {
                continue;
            }
            assert!(
                dispatched.contains(variant),
                "RelationshipKind::{variant} has no entry in RELATIONSHIP_DISPATCH — it can never be extracted from the AST"
            );
        }
    }

    #[test]
    fn test_structural_kinds_are_valid_relationship_kinds() {
        let names: HashSet<&str> = RELATIONSHIP_KIND_NAMES.iter().copied().collect();
        for kind in STRUCTURAL_RELATIONSHIP_KINDS {
            assert!(
                names.contains(kind),
                "STRUCTURAL_RELATIONSHIP_KINDS contains '{kind}' which is not in RELATIONSHIP_KIND_NAMES"
            );
        }
    }

    const INTENTIONAL_NO_LAYER: &[&str] = &[
        "package_definition",
        "library_package",
        "metadata_definition",
        "metadata_usage",
        "enumeration_definition",
        "enumeration_usage",
        "view_definition",
        "view_usage",
        "viewpoint_definition",
        "generic_usage",
    ];

    #[test]
    fn test_classify_layer_covers_all_element_kinds() {
        let no_layer: HashSet<&str> = INTENTIONAL_NO_LAYER.iter().copied().collect();
        for kind in ELEMENT_KIND_NAMES {
            let layer = classify_layer(kind);
            if layer.is_none() {
                assert!(
                    no_layer.contains(kind),
                    "Element kind '{kind}' returns None from classify_layer but is not in INTENTIONAL_NO_LAYER — add it to classify_layer or to the allowlist"
                );
            }
        }
    }

    #[test]
    fn test_intentional_no_layer_entries_are_valid() {
        for kind in INTENTIONAL_NO_LAYER {
            assert!(
                ELEMENT_KIND_NAMES.contains(kind),
                "INTENTIONAL_NO_LAYER contains '{kind}' which is not in ELEMENT_KIND_NAMES"
            );
            assert!(
                classify_layer(kind).is_none(),
                "INTENTIONAL_NO_LAYER contains '{kind}' but classify_layer returns Some — remove it from the allowlist"
            );
        }
    }

    #[test]
    fn test_parsed_element_kinds_in_vocabulary() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let known: HashSet<&str> = ELEMENT_KIND_NAMES.iter().copied().collect();
        let parsed_kinds: HashSet<&str> = graph.elements.iter().map(|e| e.kind.as_str()).collect();
        let missing: Vec<&&str> = parsed_kinds
            .iter()
            .filter(|k| !known.contains(**k))
            .collect();
        assert!(
            missing.is_empty(),
            "Eve corpus contains element kinds not in ELEMENT_KIND_NAMES: {:?}",
            missing
        );
    }

    #[test]
    fn test_parsed_relationship_kinds_in_vocabulary() {
        let results = parse_all_eve();
        let mut graph = SysmlGraph::new();
        graph.index(results).unwrap();
        let known: HashSet<&str> = RELATIONSHIP_KIND_NAMES.iter().copied().collect();
        let parsed_kinds: HashSet<&str> = graph
            .relationships
            .iter()
            .map(|r| r.kind.as_str())
            .collect();
        for kind in &parsed_kinds {
            assert!(
                known.contains(kind),
                "Eve corpus contains relationship kind '{kind}' not in RELATIONSHIP_KIND_NAMES"
            );
        }
    }
}

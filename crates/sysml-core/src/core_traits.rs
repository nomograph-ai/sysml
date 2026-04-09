use std::any::Any;
use std::path::Path;

use crate::core_error::IndexError;
use crate::core_types::{
    CheckType, DetailLevel, Finding, ParseResult, Predicate, ScoredResult, ScoringCandidate,
    SearchResult, Span, TraceOptions, TraceResult, Triple,
};

pub trait Element: Send + Sync {
    fn qualified_name(&self) -> &str;
    fn kind(&self) -> &str;
    fn file_path(&self) -> &Path;
    fn span(&self) -> Span;
    fn metadata(&self) -> &dyn Any;
}

pub trait Relationship: Send + Sync {
    fn source(&self) -> &str;
    fn target(&self) -> &str;
    fn kind(&self) -> &str;
    fn file_path(&self) -> &Path;
    fn span(&self) -> Span;
}

pub trait Parser: Send + Sync {
    type Elem: Element;
    type Rel: Relationship;
    type Error: std::error::Error;

    fn parse(
        &self,
        source: &str,
        path: &Path,
    ) -> Result<ParseResult<Self::Elem, Self::Rel>, Self::Error>;
    fn validate(&self, source: &str) -> Vec<crate::core_types::Diagnostic>;
}

pub trait KnowledgeGraph: Send + Sync {
    type Elem: Element;
    type Rel: Relationship;

    fn index(&mut self, results: Vec<ParseResult<Self::Elem, Self::Rel>>)
        -> Result<(), IndexError>;
    fn search(&self, query: &str, level: DetailLevel, limit: usize) -> Vec<SearchResult>;
    fn trace(&self, element: &str, opts: TraceOptions) -> TraceResult;
    fn check(&self, check_type: CheckType) -> Vec<Finding>;
    fn query(&self, predicate: Predicate) -> Vec<Triple>;
    fn elements(&self) -> &[Self::Elem];
    fn relationships(&self) -> &[Self::Rel];
}

pub trait Vocabulary: Send + Sync {
    fn expand_kind(&self, kind: &str) -> Vec<&str>;
    fn normalize_kind<'a>(&self, kind: &'a str) -> &'a str;
    fn relationship_kinds(&self) -> &[&str];
    fn element_kinds(&self) -> &[&str];
}

pub trait Scorer: Send + Sync {
    fn score(&self, query: &str, candidates: &[ScoringCandidate]) -> Vec<ScoredResult>;
    fn signals(&self) -> &[&str];
}

pub mod core_error;
pub mod core_traits;
pub mod core_types;

pub mod badge;
pub mod diff;
pub mod element;
pub mod graph;
pub mod metamodel;
pub mod parser;
pub mod plan;
pub mod relationship;
pub mod render;
pub mod resolve;
pub mod scaffold;
#[cfg(feature = "vector")]
pub mod vector;
pub mod vocabulary;
pub mod walker;

pub use core_error::{CoreError, IndexError};
pub use core_traits::{Element, KnowledgeGraph, Parser, Relationship, Scorer, Vocabulary};
pub use core_types::{
    CheckType, DetailLevel, Diagnostic, Direction, Finding, ParseResult, Predicate, ScoredResult,
    ScoringCandidate, SearchResult, Severity, Span, TraceFormat, TraceHop, TraceOptions,
    TraceResult, Triple,
};

pub use element::SysmlElement;
pub use graph::SysmlGraph;
pub use parser::SysmlParser;
pub use relationship::SysmlRelationship;
pub use vocabulary::{expand_query, ExpandedQuery, SysmlVocabulary};

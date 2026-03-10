use nomograph_core::traits::Relationship;
use nomograph_core::types::Span;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SysmlRelationship {
    pub source: String,
    pub target: String,
    pub kind: String,
    pub file_path: PathBuf,
    pub span: Span,
}

impl Relationship for SysmlRelationship {
    fn source(&self) -> &str {
        &self.source
    }

    fn target(&self) -> &str {
        &self.target
    }

    fn kind(&self) -> &str {
        &self.kind
    }

    fn file_path(&self) -> &Path {
        &self.file_path
    }

    fn span(&self) -> Span {
        self.span.clone()
    }
}

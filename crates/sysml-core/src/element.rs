use crate::core_traits::Element;
use crate::core_types::Span;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RflpLayer {
    Requirements,
    Functional,
    Logical,
    Physical,
}

impl fmt::Display for RflpLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RflpLayer::Requirements => write!(f, "R"),
            RflpLayer::Functional => write!(f, "F"),
            RflpLayer::Logical => write!(f, "L"),
            RflpLayer::Physical => write!(f, "P"),
        }
    }
}

impl FromStr for RflpLayer {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "R" | "REQUIREMENTS" => Ok(RflpLayer::Requirements),
            "F" | "FUNCTIONAL" => Ok(RflpLayer::Functional),
            "L" | "LOGICAL" => Ok(RflpLayer::Logical),
            "P" | "PHYSICAL" => Ok(RflpLayer::Physical),
            _ => Err(format!("Unknown RFLP layer: '{}'. Valid: R, F, L, P", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SysmlElement {
    pub qualified_name: String,
    pub kind: String,
    pub file_path: PathBuf,
    pub span: Span,
    pub doc: Option<String>,
    pub attributes: Vec<(String, String)>,
    #[serde(default)]
    pub members: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<RflpLayer>,
}

impl Element for SysmlElement {
    fn qualified_name(&self) -> &str {
        &self.qualified_name
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

    fn metadata(&self) -> &dyn Any {
        self
    }
}

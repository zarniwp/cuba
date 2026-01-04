use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use unicode_normalization::UnicodeNormalization;

use crate::shared::npath::{Rel, UNPath};

/// Defines a `GlobMatcher`
pub struct GlobMatcher {
    patterns: Vec<String>,
    globset: GlobSet,
}

/// Methods of `GlobMatcher`
impl GlobMatcher {
    /// Creates a new `GlobMatcher`
    pub fn new(patterns: &Vec<String>) -> Result<Self, globset::Error> {
        let mut builder = GlobSetBuilder::new();

        for pattern in patterns {
            let glob = GlobBuilder::new(pattern).literal_separator(true).build()?;
            builder.add(glob);
        }

        let globset = builder.build()?;

        Ok(GlobMatcher {
            patterns: patterns.to_vec(),
            globset,
        })
    }

    /// Returns a `IncludeMatcher`
    pub fn include_matcher(&self) -> IncludeMatcher {
        IncludeMatcher {
            patterns: self.patterns.clone(),
            globset: self.globset.clone(),
        }
    }

    /// Returns a `ExcludeMatcher`
    pub fn exclude_matcher(&self) -> ExcludeMatcher {
        ExcludeMatcher {
            globset: self.globset.clone(),
        }
    }
}

/// Defines a `IncludeMatcher`
pub struct IncludeMatcher {
    patterns: Vec<String>,
    globset: GlobSet,
}

/// Methods of `IncludeMatcher`
impl IncludeMatcher {
    /// Returns true if a pattern matches `path`
    pub fn is_match(&self, path: &UNPath<Rel>) -> bool {
        if self.globset.is_match(path.to_path()) {
            true
        } else {
            for pattern in &self.patterns {
                if pattern.nfc().to_string().starts_with(path.to_nfc()) {
                    return true;
                }
            }

            false
        }
    }
}

/// Defines a `ExcludeMatcher`
pub struct ExcludeMatcher {
    globset: GlobSet,
}

/// Methods of `ExcludeMatcher`
impl ExcludeMatcher {
    /// Returns true if a pattern matches `path`    
    pub fn is_match(&self, path: &UNPath<Rel>) -> bool {
        self.globset.is_match(path.to_path())
    }
}

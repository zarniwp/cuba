use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::ffi::OsStr;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

static UNIX_ROOT: Lazy<Regex> = Lazy::new(|| Regex::new(r"^/").unwrap());
static WINDOWS_DRIVE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z]:").unwrap());
static URL_SCHEME: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z][a-zA-Z0-9+\-.]*:/").unwrap());

/// Define a `NPathRoot`.
#[derive(Error, Debug)]
pub enum NPathRoot {
    Unix,
    WindowsDrive(String),
    UrlScheme(String),
    Invalid,
}

/// Impls for `NPathRoot`.
impl NPathRoot {
    /// Returns the `NPathRoot` content.
    pub fn content(&self) -> &str {
        match &self {
            NPathRoot::Unix => "",
            NPathRoot::WindowsDrive(drive) => drive,
            NPathRoot::UrlScheme(scheme) => scheme,
            NPathRoot::Invalid => "NPathRoot::Invalid",
        }
    }
}

/// Impl `Display` for `NPathRoot`.
impl fmt::Display for NPathRoot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.content())
    }
}

/// Defines a `NPathComponent`.
#[derive(Error, Debug)]
pub enum NPathComponent {
    Root(NPathRoot),
    Normal(String),
}

/// Impls for `NPathComponent`
impl NPathComponent {
    /// Returns the `NPathComponent` content.
    pub fn content(&self) -> &str {
        match &self {
            NPathComponent::Root(root) => root.content(),
            NPathComponent::Normal(segment) => segment,
        }
    }
}

/// Impl `Display` for `NPathComponent`.
impl fmt::Display for NPathComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.content())
    }
}

/// Normalizes a path to canonical internal representation with separator `/`.
fn normalize_path(path: &str) -> String {
    let normalized_path: String = path.replace('\\', "/");

    // trim trailing slash.
    normalized_path.trim_end_matches('/').to_string()
}

/// Checks, if a path has a root.
fn has_root(normalize_path: &str) -> bool {
    if UNIX_ROOT.is_match(normalize_path) || WINDOWS_DRIVE.is_match(normalize_path) {
        true
    } else {
        URL_SCHEME.is_match(normalize_path)
    }
}

/// An absolute path must have a root or be empty.
pub enum Abs {}

/// A relative path must have no root.
pub enum Rel {}

/// A file path must target to a file.
pub enum File {}

/// A dir path must target to a directory.
pub enum Dir {}

/// Defines a `NPathError`.
#[derive(Error, Debug)]
pub enum NPathError {
    #[error("Path is not absolut")]
    NoAbsPath,

    #[error("Path is not relative")]
    NoRelPath,

    #[error("Invalid operation")]
    InvalidOperation,
}

/// Defines a `UNPath<K>`
///
/// A union of normalized paths. Can hold either a Rel/Abs `NPath<Dir>` or `NPath<File>`.
///
/// With operations:
/// `UNPath<Rel> = UNPath<Abs> - NPath<Abs, Dir>`
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", content = "path")]
#[serde(bound(serialize = "NPath<K, File>: Serialize, NPath<K, Dir>: Serialize"))]
#[serde(bound(deserialize = "NPath<K, File>: Deserialize<'de>, NPath<K, Dir>: Deserialize<'de>"))]
pub enum UNPath<K> {
    File(NPath<K, File>),
    Dir(NPath<K, Dir>),
}

/// Impl of `Debug` for an absolute `UNPath`.
impl fmt::Debug for UNPath<Abs> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UNPath::File(abs_file_path) => write!(f, "abs:file:{}", abs_file_path.path_raw),
            UNPath::Dir(abs_dir_path) => write!(f, "abs:dir:{}", abs_dir_path.path_raw),
        }
    }
}

/// Impl of `Debug` for a relative `UNPath`.
impl fmt::Debug for UNPath<Rel> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UNPath::File(rel_file_path) => write!(f, "rel:file:{}", rel_file_path.path_raw),
            UNPath::Dir(rel_dir_path) => write!(f, "rel:dir:{}", rel_dir_path.path_raw),
        }
    }
}

/// Impl of `Display` for an absolute `UNPath`.
impl fmt::Display for UNPath<Abs> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UNPath::File(abs_file_path) => write!(f, "abs:file:{}", abs_file_path.path_raw),
            UNPath::Dir(abs_dir_path) => write!(f, "abs:dir:{}", abs_dir_path.path_raw),
        }
    }
}

/// Impl of `Display` for a relative `UNPath`.
impl fmt::Display for UNPath<Rel> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UNPath::File(rel_file_path) => write!(f, "rel:file:{}", rel_file_path.path_raw),
            UNPath::Dir(rel_dir_path) => write!(f, "rel:dir:{}", rel_dir_path.path_raw),
        }
    }
}

/// Impl of `FromStr` for an absolute `UNPath`.
impl FromStr for UNPath<Abs> {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        if let Some(path) = string.strip_prefix("abs:file:") {
            Ok(UNPath::File(
                NPath::<Abs, File>::try_from(path).map_err(|err| err.to_string())?,
            ))
        } else if let Some(path) = string.strip_prefix("abs:dir:") {
            Ok(UNPath::Dir(
                NPath::<Abs, Dir>::try_from(path).map_err(|err| err.to_string())?,
            ))
        } else {
            Err(format!("Invalid UNPath<Abs> string: {}", string))
        }
    }
}

/// Impl of `FromStr` for a relative `UNPath`.
impl FromStr for UNPath<Rel> {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        if let Some(path) = string.strip_prefix("rel:file:") {
            Ok(UNPath::File(
                NPath::<Rel, File>::try_from(path).map_err(|err| err.to_string())?,
            ))
        } else if let Some(path) = string.strip_prefix("rel:dir:") {
            Ok(UNPath::Dir(
                NPath::<Rel, Dir>::try_from(path).map_err(|err| err.to_string())?,
            ))
        } else {
            Err(format!("Invalid UNPath<Rel> string: {}", string))
        }
    }
}

/// Methods of `UNPath`
impl<K> UNPath<K> {
    /// Returns the `UNPath` as a path.
    pub fn to_path(&self) -> &Path {
        match self {
            UNPath::File(file_path) => file_path.to_path(),
            UNPath::Dir(dir_path) => dir_path.to_path(),
        }
    }

    /// Returns true, if the `UNPath` ends with the relative file `NPath`.
    pub fn ends_with_file(&self, rel_file_path: &NPath<Rel, File>) -> bool {
        match self {
            UNPath::File(file_path) => file_path.ends_with(rel_file_path),
            _ => false,
        }
    }

    /// Returns true, if the `UNPath` ends with the relative directory `NPath`.
    pub fn ends_with_dir(&self, rel_dir_path: &NPath<Rel, Dir>) -> bool {
        match self {
            UNPath::Dir(dir_path) => dir_path.ends_with(rel_dir_path),
            _ => false,
        }
    }

    /// Returns `NPath<K, File>` if file; otherwise returns `FnOnce`.
    pub fn file_or_else<F: FnOnce() -> NPath<K, File>>(self, op: F) -> NPath<K, File> {
        match self {
            UNPath::File(file_path) => file_path,
            UNPath::Dir(_dir_path) => op(),
        }
    }

    /// Returns `NPath<K, Dir>` if directory; otherwise returns `FnOnce`.
    pub fn dir_or_else<F: FnOnce() -> NPath<K, Dir>>(self, op: F) -> NPath<K, Dir> {
        match self {
            UNPath::File(_file_path) => op(),
            UNPath::Dir(dir_path) => dir_path,
        }
    }

    /// Returns true if the `UNPath` is a file path.
    pub fn is_file(&self) -> bool {
        match self {
            UNPath::File(_file_path) => true,
            UNPath::Dir(_dir_path) => false,
        }
    }

    /// Returns true if the `UNPath` is a directory path.
    pub fn is_dir(&self) -> bool {
        match self {
            UNPath::File(_file_path) => false,
            UNPath::Dir(_dir_path) => true,
        }
    }

    /// Returns the `UNPath` as raw str.
    pub fn to_raw(&self) -> &str {
        match self {
            UNPath::File(file_path) => file_path.to_raw(),
            UNPath::Dir(dir_path) => dir_path.to_raw(),
        }
    }

    /// Returns the `UNPath` as nfc str.
    pub fn to_nfc(&self) -> &str {
        match self {
            UNPath::File(file_path) => file_path.to_nfc(),
            UNPath::Dir(dir_path) => dir_path.to_nfc(),
        }
    }
}

/// Methods of an absolute `UNPath`.
impl UNPath<Abs> {
    /// Returns the absolut path as os `PathBuf`.
    pub fn as_os_path(&self) -> PathBuf {
        match self {
            UNPath::File(abs_file_path) => abs_file_path.as_os_path(),
            UNPath::Dir(abs_dir_path) => abs_dir_path.as_os_path(),
        }
    }

    /// `UNPath<Rel> = UNPath<Abs> - NPath<Abs, Dir>`
    pub fn sub_abs_dir(&self, abs_dir_path: &NPath<Abs, Dir>) -> Result<UNPath<Rel>, NPathError> {
        match self {
            UNPath::File(self_abs_file_path) => {
                match self_abs_file_path.sub_abs_dir(abs_dir_path) {
                    Ok(rel_path) => Ok(rel_path.into()),
                    Err(err) => Err(err),
                }
            }
            UNPath::Dir(self_abs_dir_path) => match self_abs_dir_path.sub_abs_dir(abs_dir_path) {
                Ok(rel_path) => Ok(rel_path.into()),
                Err(err) => Err(err),
            },
        }
    }

    /// Returns the path components of the absolute `UNPath`.
    pub fn components(&self) -> Box<dyn Iterator<Item = NPathComponent> + '_> {
        match self {
            UNPath::File(file_path) => Box::new(file_path.components()),
            UNPath::Dir(dir_path) => Box::new(dir_path.components()),
        }
    }
}

/// Methods of a relative `UNPath`.
impl UNPath<Rel> {
    /// Returns the path components.
    pub fn components(&self) -> Box<dyn Iterator<Item = NPathComponent> + '_> {
        match self {
            UNPath::File(file_path) => Box::new(file_path.components()),
            UNPath::Dir(dir_path) => Box::new(dir_path.components()),
        }
    }
}

/// Impl of `From` (clone) for a file `UNPath`.
impl<K> From<&NPath<K, File>> for UNPath<K>
where
    NPath<K, File>: Clone,
{
    fn from(path: &NPath<K, File>) -> Self {
        UNPath::File(path.clone())
    }
}

/// Impl of `From` (clone) for a directory `UNPath`.
impl<K> From<&NPath<K, Dir>> for UNPath<K>
where
    NPath<K, Dir>: Clone,
{
    fn from(path: &NPath<K, Dir>) -> Self {
        UNPath::Dir(path.clone())
    }
}

/// Impl of `From` for a file `UNPath`.
impl<K> From<NPath<K, File>> for UNPath<K> {
    fn from(path: NPath<K, File>) -> Self {
        UNPath::File(path)
    }
}

/// Impl of `From` for a directory `UNPath`.
impl<K> From<NPath<K, Dir>> for UNPath<K> {
    fn from(path: NPath<K, Dir>) -> Self {
        UNPath::Dir(path)
    }
}

/// Impl of `Clone` for `UNPath`.
impl<K> Clone for UNPath<K> {
    fn clone(&self) -> Self {
        match self {
            UNPath::File(file_path) => UNPath::File(file_path.clone()),
            UNPath::Dir(dir_path) => UNPath::Dir(dir_path.clone()),
        }
    }
}

/// Impl of `PartialEq` for `UNPath`.
impl<K1, K2> PartialEq<UNPath<K2>> for UNPath<K1> {
    fn eq(&self, other: &UNPath<K2>) -> bool {
        match (self, other) {
            (UNPath::File(file_path_1), UNPath::File(file_path_2)) => file_path_1 == file_path_2,
            (UNPath::Dir(dir_path_1), UNPath::Dir(dir_path_2)) => dir_path_1 == dir_path_2,
            _ => false,
        }
    }
}

/// Impl of `Eq` for `UNPath`.
impl<K> Eq for UNPath<K> {}

/// Impl of `Hash` for `UNPath`.
impl<K> Hash for UNPath<K> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            UNPath::File(file_path) => {
                0u8.hash(state); // variant discriminant
                file_path.hash(state);
            }
            UNPath::Dir(dir_path) => {
                1u8.hash(state);
                dir_path.hash(state);
            }
        }
    }
}

/// Defines a `NPath<K, T>`.
///
/// `NPath` ensures a normalized pattern for paths.
/// Conventions:
///
/// - separates its elements with '/'
/// - no trailing `/`
///
/// Operations:
/// `NPath<Abs, Dir> = NPath<Abs, Dir> + NPath<Rel, Dir>`
/// `NPath<Abs, File> = NPath<Abs, Dir> + NPath<Rel, File>`
/// `NPath<Abs, Dir> = NPath<Abs, Dir> - NPath<Rel, Dir>`
/// `NPath<Abs, Dir> = NPath<Abs, File> - NPath<Rel, File>`
/// `NPath<Rel, T> = NPath<Abs, T> - NPath<Abs, Dir>`
pub struct NPath<K, T> {
    path_raw: String,
    path_nfc: String, // Only use for comparsion and hashing.
    _marker: PhantomData<(K, T)>,
}

/// Impl `TryFrom` for an absolute `NPath`.
impl<T> TryFrom<&str> for NPath<Abs, T> {
    type Error = NPathError;

    fn try_from(path: &str) -> Result<Self, Self::Error> {
        let normalized_path = normalize_path(path);

        if has_root(&normalized_path) || normalized_path.is_empty() {
            Ok(NPath::new(&normalized_path))
        } else {
            Err(NPathError::NoAbsPath)
        }
    }
}

/// Impl `TryFrom` for an absolute `NPath`.
impl<T> TryFrom<String> for NPath<Abs, T> {
    type Error = NPathError;

    fn try_from(path: String) -> Result<Self, Self::Error> {
        let normalized_path = normalize_path(&path);

        if has_root(&normalized_path) || normalized_path.is_empty() {
            Ok(NPath::new(&normalized_path))
        } else {
            Err(NPathError::NoAbsPath)
        }
    }
}

/// Impl `TryFrom` for a relative `NPath`.
impl<T> TryFrom<&str> for NPath<Rel, T> {
    type Error = NPathError;

    fn try_from(path: &str) -> Result<Self, Self::Error> {
        let normalized_path = normalize_path(path);

        if !has_root(&normalized_path) || normalized_path.is_empty() {
            Ok(NPath::new(&normalized_path))
        } else {
            Err(NPathError::NoRelPath)
        }
    }
}

/// Impl `TryFrom` for a relative `NPath`.
impl<T> TryFrom<String> for NPath<Rel, T> {
    type Error = NPathError;

    fn try_from(path: String) -> Result<Self, Self::Error> {
        let normalized_path = normalize_path(&path);

        if !has_root(&normalized_path) || normalized_path.is_empty() {
            Ok(NPath::new(&normalized_path))
        } else {
            Err(NPathError::NoRelPath)
        }
    }
}

/// Impl of `Debug` for `NPath`.
impl<K, T> fmt::Debug for NPath<K, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_nfc())
    }
}

/// Impl of `Display` for `NPath`.
impl<K, T> fmt::Display for NPath<K, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_nfc())
    }
}

/// Methods for `NPath`.
impl<K, T> NPath<K, T> {
    /// Create a new `NPath`, only for internal use.
    fn new(path_str: &str) -> Self {
        let path_raw = path_str.to_string();
        let path_nfc: String = path_str.nfc().collect(); // only for equality & hashing
        Self {
            path_raw,
            path_nfc,
            _marker: PhantomData,
        }
    }

    /// Returns true, if the `NPath` ends with `rel_path`.
    pub fn ends_with(&self, rel_path: &NPath<Rel, T>) -> bool {
        self.path_raw.ends_with(&rel_path.path_raw)
    }

    /// Clears the `NPath`.
    pub fn clear(&mut self) {
        self.path_raw.clear();
        self.path_nfc.clear();
    }

    /// Returns true if the `NPath` is empty.
    pub fn is_empty(&self) -> bool {
        self.path_raw.is_empty()
    }

    /// Returns the `NPath` as path.
    pub fn to_path(&self) -> &Path {
        Path::new(&self.path_raw)
    }

    /// Returns the `NPath` as raw str.
    pub fn to_raw(&self) -> &str {
        &self.path_raw
    }

    /// Returns the `NPath` as nfc str.
    pub fn to_nfc(&self) -> &str {
        &self.path_nfc
    }
}

/// Impl of `Clone` for `NPath`.
impl<K, T> Clone for NPath<K, T> {
    fn clone(&self) -> Self {
        NPath {
            path_raw: self.path_raw.clone(),
            path_nfc: self.path_nfc.clone(),
            _marker: PhantomData,
        }
    }
}

/// Impl of `Eq` for `NPath`.
impl<K, T> Eq for NPath<K, T> {}

// Impl of `PartialEq` for `NPath`.
impl<K1, T1, K2, T2> PartialEq<NPath<K2, T2>> for NPath<K1, T1> {
    fn eq(&self, other: &NPath<K2, T2>) -> bool {
        self.path_nfc == other.path_nfc
    }
}

/// Impl of `Hash` for `NPath`.
impl<K, T> Hash for NPath<K, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.path_nfc.hash(state);
    }
}

/// Impl of `Serialize` for `NPath`.
impl<K, T> Serialize for NPath<K, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize the inner path as a string
        serializer.serialize_str(&self.path_raw)
    }
}

/// Impl of `Deserialize` for `NPath`.
impl<'de, K, T> Deserialize<'de> for NPath<K, T>
where
    NPath<K, T>: TryFrom<String>,
    <NPath<K, T> as TryFrom<String>>::Error: std::fmt::Display,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let path_raw = String::deserialize(deserializer)?;
        NPath::<K, T>::try_from(path_raw).map_err(serde::de::Error::custom)
    }
}

/// Impl of `Default` for an absolute `NPath`.
impl<T> Default for NPath<Abs, T> {
    fn default() -> Self {
        NPath::new("")
    }
}

/// Methods of an absolute `NPath`.
impl<T> NPath<Abs, T> {
    /// Returns the absolut path as os path.
    pub fn as_os_path(&self) -> PathBuf {
        let os_string = self.path_raw.replace("/", std::path::MAIN_SEPARATOR_STR);
        PathBuf::from(os_string)
    }

    /// Returns the path components.
    pub fn components(&self) -> impl Iterator<Item = NPathComponent> + '_ {
        let path = self.path_raw.as_str();

        let (root, rest) = if let Some(expr_match) = URL_SCHEME.find(path) {
            (
                NPathComponent::Root(NPathRoot::UrlScheme(path[..expr_match.end()].into())),
                &path[expr_match.end()..],
            )
        } else if let Some(expr_match) = WINDOWS_DRIVE.find(path) {
            (
                NPathComponent::Root(NPathRoot::WindowsDrive(path[..expr_match.end()].into())),
                &path[expr_match.end()..],
            )
        } else if let Some(expr_match) = UNIX_ROOT.find(path) {
            (
                NPathComponent::Root(NPathRoot::Unix),
                &path[expr_match.end()..],
            )
        } else {
            (NPathComponent::Root(NPathRoot::Invalid), path)
        };

        std::iter::once(root).chain(
            rest.split('/')
                .filter(|s| !s.is_empty())
                .map(|s| NPathComponent::Normal(s.into())),
        )
    }

    /// `NPath<Abs, T> = NPath<Abs, T> + NPath<Abs, Dir>`
    pub fn sub_abs_dir(&self, abs_dir_path: &NPath<Abs, Dir>) -> Result<NPath<Rel, T>, NPathError> {
        match self.path_raw.strip_prefix(abs_dir_path.path_raw.as_str()) {
            Some(rel_path) => Ok(NPath::new(rel_path.trim_start_matches('/'))),
            None => Err(NPathError::InvalidOperation),
        }
    }
}

impl<T> NPath<Rel, T> {
    /// Returns the path components.
    pub fn components(&self) -> impl Iterator<Item = NPathComponent> + '_ {
        self.path_raw
            .split("/")
            .map(|segment| NPathComponent::Normal(segment.into()))
    }
}

/// Methods of an absolute directory `NPath`.
impl NPath<Abs, Dir> {
    /// `NPath<Abs, Dir> = NPath<Abs, Dir> + NPath<Rel, Dir>`
    pub fn add_rel_dir(&self, rel_dir_path: &NPath<Rel, Dir>) -> NPath<Abs, Dir> {
        NPath::new(&(self.path_raw.clone() + "/" + &rel_dir_path.path_raw))
    }

    /// `NPath<Abs, File> = NPath<Abs, Dir> + NPath<Rel, File>`
    pub fn add_rel_file(&self, rel_file_path: &NPath<Rel, File>) -> NPath<Abs, File> {
        NPath::new(&(self.path_raw.clone() + "/" + &rel_file_path.path_raw))
    }

    /// `NPath<Abs, Dir> = NPath<Abs, Dir> - NPath<Rel, Dir>`
    pub fn sub_rel_dir(
        &self,
        rel_dir_path: &NPath<Rel, Dir>,
    ) -> Result<NPath<Abs, Dir>, NPathError> {
        match self.path_raw.strip_suffix(rel_dir_path.path_raw.as_str()) {
            Some(abs_dir_path) => Ok(NPath::new(abs_dir_path.trim_end_matches('/'))),
            None => Err(NPathError::InvalidOperation),
        }
    }

    /// Union of an absolute directory `NPath` and a relative `UNPath`.
    pub fn union(&self, rel_path: &UNPath<Rel>) -> Result<UNPath<Abs>, NPathError> {
        let mut union_path = String::new();

        let abs_components: Vec<NPathComponent> = self.components().collect();
        let rel_components: Vec<NPathComponent> = rel_path.components().collect();

        let mut abs_idx: usize = 0;
        let mut rel_idx: usize = 0;

        let mut abs_done = false;
        let mut rel_done = false;

        loop {
            if abs_components[abs_idx].content().nfc().to_string()
                == rel_components[rel_idx].content().nfc().to_string()
            {
                union_path.push_str(abs_components[abs_idx].content());
                union_path.push('/');

                if abs_idx + 1 < abs_components.len() {
                    abs_idx += 1;
                }
                if rel_idx + 1 < rel_components.len() {
                    rel_idx += 1;
                }
            } else if rel_idx == 0 {
                union_path.push_str(abs_components[abs_idx].content());
                union_path.push('/');

                if abs_idx + 1 < abs_components.len() {
                    abs_idx += 1;
                }
            } else {
                union_path.push_str(rel_components[rel_idx].content());
                union_path.push('/');

                if rel_idx + 1 < rel_components.len() {
                    rel_idx += 1;
                }
            }

            if abs_done && rel_idx == 0 {
                break;
            }

            if abs_done && rel_done {
                break;
            }

            if abs_idx + 1 == abs_components.len() {
                abs_done = true;
            }
            if rel_idx + 1 == rel_components.len() {
                rel_done = true;
            }
        }

        match rel_path {
            UNPath::File(_rel_file_path) => {
                let abs_file_path = NPath::<Abs, File>::try_from(union_path)?;
                Ok(UNPath::File(abs_file_path))
            }
            UNPath::Dir(_rel_dir_path) => {
                let abs_dir_path = NPath::<Abs, Dir>::try_from(union_path)?;
                Ok(UNPath::Dir(abs_dir_path))
            }
        }
    }
}

/// Methods of an absolute file `NPath`.
impl NPath<Abs, File> {
    /// `NPath<Abs, File> = NPath<Abs, File> - NPath<Rel, File>`
    pub fn sub_rel_file(
        &self,
        rel_file_path: &NPath<Rel, File>,
    ) -> Result<NPath<Abs, File>, NPathError> {
        match self.path_raw.strip_suffix(rel_file_path.path_raw.as_str()) {
            Some(abs_file_path) => Ok(NPath::new(abs_file_path.trim_end_matches('/'))),
            None => Err(NPathError::InvalidOperation),
        }
    }
}

/// Impl of `Default` for a relative `NPath`.
impl<T> Default for NPath<Rel, T> {
    fn default() -> Self {
        NPath::new("")
    }
}

impl<K> NPath<K, File> {
    /// Pushes an extension to the file `NPath`.
    pub fn push_extension(&mut self, extension: &str) {
        *self = NPath::new(&(self.path_raw.clone() + "." + extension))
    }

    /// Pops (removes) an extension from the file `NPath`.
    pub fn pop_extension(&mut self) -> bool {
        match self.extension() {
            Some(ext) => {
                if let Some(ext_str) = ext.to_str() {
                    let suffix = format!(".{}", ext_str);
                    if let Some(stripped) = self.path_raw.strip_suffix(&suffix) {
                        *self = NPath::new(stripped);
                        return true;
                    }
                }
                false
            }
            None => false,
        }
    }

    /// Pops (removes) an extension from the file `NPath` if it is extension.
    pub fn pop_extension_if(&mut self, extension: &str) -> bool {
        match self.extension() {
            Some(ext) => {
                if ext == extension {
                    self.pop_extension()
                } else {
                    false
                }
            }
            None => false,
        }
    }

    /// Returns the extension of the file `NPath`.
    pub fn extension(&self) -> Option<&OsStr> {
        Path::new(&self.path_raw).extension()
    }
}

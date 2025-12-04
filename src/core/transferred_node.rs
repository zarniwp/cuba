#![allow(unused)]

use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use serde_with::DisplayFromStr;
use serde_with::serde_as;
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use crate::shared::npath::{Dir, File, NPath, Rel, UNPath};

/// Defines the `Flags` for transferred nodes.
bitflags! {
    #[derive(PartialEq, Hash, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
    pub struct Flags: u8 {
        const COMPRESSED    = 0b00000001;
        const ENCRYPTED     = 0b00000010;
        const VERIFIED      = 0b00000100;
        const VERIFY_ERROR  = 0b00001000;
        const ORPHAN        = 0b00010000;
    }
}

/// Defines the `MatchMode` for masked flags.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum MatchMode {
    Eq,
    Uq,
}

// Defines the `MaskedFlags`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct MaskedFlags {
    match_mode: MatchMode,
    flags: Flags,
    mask: Flags,
}

/// Methods of `MaskedFlags`.
impl MaskedFlags {
    /// Creates a new `MaskedFlags`.
    pub fn new() -> Self {
        Self {
            match_mode: MatchMode::Eq,
            flags: Flags::empty(),
            mask: Flags::empty(),
        }
    }

    /// Returns the `Flags`.
    pub fn flags(&self) -> Flags {
        self.flags
    }

    /// Inserts `other` in `Flags`.
    pub fn insert(&mut self, other: Flags) -> &Self {
        self.flags.insert(other);
        self
    }

    #[allow(unused)]
    /// Inserts `other` in `Flags``.
    pub fn remove(&mut self, other: Flags) -> &Self {
        self.flags.remove(other);
        self
    }

    /// Returns true if `Flags` contains `other`.
    pub fn contains(&self, other: Flags) -> bool {
        self.flags.contains(other)
    }

    /// With mode.
    pub fn with_mode(&self, mode: MatchMode) -> Self {
        Self {
            match_mode: mode,
            flags: self.flags,
            mask: self.mask,
        }
    }

    /// With flags.
    pub fn with_flags(&self, flags: Flags) -> Self {
        Self {
            match_mode: self.match_mode,
            flags,
            mask: self.flags,
        }
    }

    /// With mask.
    pub fn with_mask(&self, mask: Flags) -> Self {
        Self {
            match_mode: self.match_mode,
            flags: self.flags,
            mask,
        }
    }

    /// Returns true if masked bits match `other`.
    pub fn matches(&self, other: Flags) -> bool {
        match self.match_mode {
            MatchMode::Eq => (self.flags & self.mask) == (other & self.mask),
            MatchMode::Uq => (self.flags & self.mask) != (other & self.mask),
        }
    }

    /// Apply masked bits from self.flags into `other`, preserving bits outside the mask.
    pub fn apply(&self, other: &mut Flags) {
        // Clear masked bits.
        *other &= !self.mask;
        // Write masked flags.
        *other |= self.flags & self.mask;
    }
}

/// Impl `Default` for `MaskedFlags`.
impl Default for MaskedFlags {
    fn default() -> Self {
        Self::new()
    }
}

/// Compares two optional signatures.
pub fn sig_valid_and_match(sig_a: Option<[u8; 32]>, sig_b: Option<[u8; 32]>) -> bool {
    match (sig_a, sig_b) {
        (None, None) => false,
        (Some(sig_a), Some(sig_b)) => sig_a == sig_b,
        _ => false,
    }
}

/// Defines a `TransferredNode`.
/// 
/// Structure that holds information about a transferred node.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferredNode {
    // The rel path of the dest node.
    dest_rel_path: UNPath<Rel>,

    // The flags.
    pub flags: Flags,

    /// The password id, if encrypted.
    pub password_id: Option<String>,

    // The signature of the src node.
    pub src_signature: Option<[u8; 32]>,
}

/// Methods of `TransferredNode`.
impl TransferredNode {
    /// Creates a new `TransferredNode` instance from a file.
    pub fn from_file(
        path: &NPath<Rel, File>,
        flags: Flags,
        password_id: Option<String>,
        src_signature: &[u8; 32],
    ) -> Self {
        Self {
            dest_rel_path: path.into(),
            flags,
            password_id,
            src_signature: Some(*src_signature),
        }
    }

    /// Creates a new `TransferredNode` instance from a dir.
    pub fn from_dir(path: &NPath<Rel, Dir>, flags: Flags) -> Self {
        Self {
            dest_rel_path: path.into(),
            flags,
            password_id: None,
            src_signature: None,
        }
    }
}

/// Defines the `TransferredNodes`.
/// 
/// A Map that holds all transferred nodes.
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferredNodes(
    #[serde_as(as = "HashMap<DisplayFromStr, _>")] pub HashMap<UNPath<Rel>, TransferredNode>,
);

/// Impl `Default` for `TransferredNodes`.
impl Default for TransferredNodes {
    fn default() -> Self {
        Self::new()
    }
}

/// Methods of `TransferredNodes`.
impl TransferredNodes {
    /// Create new `TransferredNodes`
    pub fn new() -> Self {
        TransferredNodes(HashMap::new())
    }

    /// Insert `flags` for all transferred nodes.
    pub fn insert_flags(&mut self, flags: Flags) {
        for node in self.values_mut() {
            node.flags.insert(flags)
        }
    }

    /// Removes `flags` for all transferred nodes.
    pub fn remove_flags(&mut self, flags: Flags) {
        for node in self.values_mut() {
            node.flags.remove(flags)
        }
    }

    /// Returns a view over the transferred nodes.
    pub fn view<T>(&self) -> View<'_, T> {
        View {
            nodes: self,
            _marker: PhantomData,
        }
    }

    /// Returns a mut view over the transferred nodes.
    pub fn view_mut<T>(&mut self) -> ViewMut<'_, T> {
        ViewMut {
            nodes: self,
            _marker: PhantomData,
        }
    }

    /// Returns the count of nodes.
    pub fn node_count(&self) -> usize {
        self.len()
    }

    /// Removes all directories from the nodes.
    pub fn remove_dirs(&mut self) {
        let dirs_to_remove: Vec<_> = self
            .keys()
            .filter(|src_node| src_node.is_dir())
            .cloned()
            .collect();

        for src_node in dirs_to_remove {
            self.remove(&src_node);
        }
    }
}

/// Impl `Deref` for `TransferredNodes`.
impl Deref for TransferredNodes {
    type Target = HashMap<UNPath<Rel>, TransferredNode>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Impl `DerefMut` for `TransferredNodes`.
impl DerefMut for TransferredNodes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// The generic view structure parameterized by type T.
pub struct View<'a, T> {
    nodes: &'a TransferredNodes,
    _marker: PhantomData<T>,
}

/// The generic mut view structure parameterized by type T.
pub struct ViewMut<'a, T> {
    nodes: &'a mut TransferredNodes,
    _marker: PhantomData<T>,
}

pub struct Backup;
pub struct Restore;

/// Methods for view with backup type.
impl<'a> View<'a, Backup> {
    /// Returns the transferred node for the given rel src node.
    pub fn get_node_for_src(&self, src_rel_path: &UNPath<Rel>) -> Option<&TransferredNode> {
        self.nodes.get(src_rel_path)
    }

    /// Iterates over all src nodes.
    pub fn iter_src_nodes(&self) -> impl Iterator<Item = &UNPath<Rel>> {
        self.nodes.keys()
    }

    /// Returns the dest rel path.
    pub fn get_dest_rel_path(&self, node: &TransferredNode) -> UNPath<Rel> {
        node.dest_rel_path.clone()
    }
}

/// Methods for mut view with backup type.
impl<'a> ViewMut<'a, Backup> {
    /// Sets a transferred node.
    pub fn set_transferred_node(
        &mut self,
        src_rel_path: &UNPath<Rel>,
        transferred_node: &TransferredNode,
    ) {
        self.nodes
            .insert(src_rel_path.clone(), transferred_node.clone());
    }

    /// Set flags.
    pub fn set_flags(&mut self, src_rel_path: &UNPath<Rel>, flags: Flags) {
        if let Some(node) = self.nodes.get_mut(src_rel_path) {
            node.flags = flags;
        }
    }
}

/// Methods for view with restore type.
impl<'a> View<'a, Restore> {
    /// Returns the transferred node for the given rel src node.
    pub fn get_node_for_src(&self, src_rel_path: &UNPath<Rel>) -> Option<&TransferredNode> {
        self.nodes
            .values()
            .find(|node| node.dest_rel_path == *src_rel_path)
    }

    /// Iterates over all src nodes.
    pub fn iter_src_nodes(&self) -> impl Iterator<Item = &UNPath<Rel>> {
        self.nodes.values().map(|node| &node.dest_rel_path)
    }

    /// Returns the dest rel path.
    pub fn get_dest_rel_path(&self, node: &TransferredNode) -> Option<UNPath<Rel>> {
        self.nodes
            .iter()
            .find(|(_, map_node)| map_node.dest_rel_path == node.dest_rel_path)
            .map(|(key, _)| key.clone())
    }
}

/// Methods for mut view with restore type.
impl<'a> ViewMut<'a, Restore> {
    /// Sets a transferred node.
    pub fn set_transferred_node(
        &mut self,
        dest_rel_path: &UNPath<Rel>,
        transferred_node: &TransferredNode,
    ) {
        self.nodes
            .insert(dest_rel_path.clone(), transferred_node.clone());
    }

    /// Set flags.
    pub fn set_flags(&mut self, src_rel_path: &UNPath<Rel>, flags: Flags) {
        if let Some(transferred_node) = self
            .nodes
            .values_mut()
            .find(|node| node.dest_rel_path == *src_rel_path)
        {
            transferred_node.flags = flags;
        }
    }
}

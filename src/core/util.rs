use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use std::collections::VecDeque;

use crate::shared::npath::{NPath, Rel};

// Creates a globset matcher.
pub fn create_matcher(patterns: Vec<String>) -> Result<GlobSet, globset::Error> {
    let mut builder = GlobSetBuilder::new();

    for pattern in patterns {
        let glob = GlobBuilder::new(&pattern).literal_separator(true).build()?;
        builder.add(glob);
    }

    builder.build()
}

/// Move npaths with the depth n from all_paths to depth_paths.
pub fn move_rel_npaths<T>(
    all_paths: &mut VecDeque<NPath<Rel, T>>,
    depth_paths: &mut VecDeque<NPath<Rel, T>>,
    depth: usize,
) {
    let mut index = 0;

    while index < all_paths.len() {
        if all_paths[index].components().count() == depth {
            depth_paths.push_back(all_paths.remove(index).unwrap());
        } else {
            index += 1;
        }
    }
}

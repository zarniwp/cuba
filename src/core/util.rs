use std::collections::VecDeque;

use crate::shared::npath::{NPath, Rel};

/// Move `NPath` with the depth n from `all_paths` to `depth_paths`.
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

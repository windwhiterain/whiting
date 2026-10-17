//! Where a node stands in a plant: the path of child indices that reaches it.
//!
//! # The one idea
//!
//! **A node's identity is its derivation, not a number that survives an edit by accident.** A [`Handle`]
//! is the path a walk took to reach the node — `trunk`, `trunk/bough 7`, `trunk/bough 7/branch 3` — so an
//! author addresses a node the way the plant was built and a reader can see the address in a report. A
//! path that no longer resolves is then a *reported* staleness ([`crate::plant::HandleOrphaned`]) rather
//! than a number that quietly names a different branch, which is the hole a name-keyed table leaves open.
//!
//! # The names
//!
//! The first generations get the words a botanist and a modeller both use — trunk, bough, branch, twig —
//! and everything deeper is a sprig. The names are for a report's reader; the path is the data.

/// How many generations have their own name; deeper ones reuse the last.
const NAMES: [&str; 5] = ["trunk", "bough", "branch", "twig", "sprig"];

/// A node's place in a plant: the child indices from the root, in order.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Handle {
    path: Vec<u32>,
}

impl Handle {
    /// The root's handle: the empty path.
    pub fn root() -> Self {
        Self { path: Vec::new() }
    }

    /// The handle of the child at `index` of the node this handle names.
    pub fn child(&self, index: u32) -> Self {
        let mut path = self.path.clone();
        path.push(index);
        Self { path }
    }

    /// The handle of the parent node, or `None` for the root.
    pub fn parent(&self) -> Option<Self> {
        let mut path = self.path.clone();
        path.pop()?;
        Some(Self { path })
    }

    /// The child indices, from the root down.
    pub fn path(&self) -> &[u32] {
        &self.path
    }

    /// How many steps from the root: `0` for the root itself.
    pub fn depth(&self) -> usize {
        self.path.len()
    }

    /// Whether this handle names the root.
    pub fn is_root(&self) -> bool {
        self.path.is_empty()
    }

    /// The generation's name, for a report's reader.
    pub fn generation(&self) -> &'static str {
        NAMES[self.depth().min(NAMES.len() - 1)]
    }
}

impl std::fmt::Display for Handle {
    /// `trunk/bough 7/branch 3`: each step names its generation and the index it took.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", NAMES[0])?;
        for (depth, index) in self.path.iter().enumerate() {
            write!(formatter, "/{} {index}", NAMES[(depth + 1).min(NAMES.len() - 1)])?;
        }
        Ok(())
    }
}

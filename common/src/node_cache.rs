// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use crate::StringId;
use crate::repeat::{LoopReturn, repeat};
use core::fmt::{Debug, Formatter, Result};
use core::marker::PhantomData;
use core::num::NonZeroU64;
use core::mem::transmute;

pub const MAX_PATH_COMPONENTS: usize = 256;

#[cfg_attr(feature = "user", derive(Copy))]
#[derive(PartialEq, Eq, Hash, Clone)]
#[repr(C)]
pub struct PathNode {
    pub parent_id: Option<NodeId>,
    pub name_id: StringId,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct NodeId(pub NonZeroU64);

impl NodeId {
    // If code is guaranteed to be unreachable, but the compiler and verifier don't know, we
    // cannot panic as we would in user space. In eBPF we must continue somehow. This error
    // node ID can be returned in unreachable code.
    pub fn error_id() -> Self {
        // avoid all possible panics by using transmute
        NodeId(unsafe { transmute(u64::MAX) })
    }
}

impl Debug for NodeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.0.fmt(f)
    }
}

// Type parameter `S` allows us to pass a buffer and possible context to string-id generation.
pub trait PathRep<S>: Sized {
    fn name_id(&self, buffer: *mut S) -> StringId;
    fn parent(&self) -> Option<Self>;
}

/// We define a trait for the node cache in order to use the same implementation in eBPF and
/// in user space and also in tests.
/// The type `P` represents a path. It must be able to provide a parent and a StringId for its
/// name (with the help of type `S`, which may be a buffer or string to identifier mapping table).
pub trait NodeCacheTrait<P: PathRep<S>, S>: Sized {
    /// If `root_path.parent == None`, this function provides the node ID for the given `root_path`.
    fn root_node_id(&self, root_path: &P) -> Option<NodeId>;

    /// Access to a HashMap which maps PathNode to NodeId
    fn id_for_node(&self, node: &PathNode) -> Option<NodeId>;

    /// Access to a HashMap which maps NodeId to PathNode
    fn node_for_id(&self, node_id: NodeId) -> Option<PathNode>;

    /// provides a buffer used for internal computation. We request a pointer to get around
    /// borrow- and lifetime checks. The buffer must be valid while `node_id_for_path()` runs.
    fn string_id_buffer(&mut self) -> *mut [StringId; MAX_PATH_COMPONENTS];

    /// Context transparently passed to PathRep::name_id(). We need this in eBPF to provide
    /// a buffer. This API also uses a pointer to get around borrow checker troubles.
    fn name_id_context(&mut self) -> *mut S;

    /// When a new child is seen for a parent, this function inserts the new PathNode and NodeId
    /// into both HashMaps which map between these types. First insert into the node -> id table
    /// and fail if the entry already exists. The id -> node table needs no further checking.
    /// Fails with return value `false` if entry exists.
    fn insert_node(&mut self, node: &PathNode, node_id: NodeId) -> bool;

    /// Returns the next free NodeId. If called multiple times, returns the same ID.
    fn new_id(&mut self) -> NodeId;

    /// Marks the last NodeId obtained with `new_id()` as used, the next call to `new_id()` returns
    /// a different NodeId.
    fn consume_id(&mut self);

    /*
    // This recursive implementation is not used. It is here as a readable prototype. The loop
    // version below is much more complex, but suitable for eBPF.
    fn node_id_for_path_recursive(&mut self, path: P) -> NodeId {
        let parent_path = match path.parent() {
            Some(parent) => parent,
            None => return self.root_node_id(&path),
        };
        let parent_id = self.node_id_for_path_recursive(parent_path);
        let node = PathNode {
            parent_id: Some(parent_id),
            name_id: path.name_id(),
        };
        if let Some(node_id) = self.id_for_node(&node) {
            return node_id;
        } else {
            let new_id = self.new_id();
            if self.insert_node(&node, new_id) {
                self.consume_id();
                return new_id;
            } else {
                return self.id_for_node(&node).unwrap_or(NodeId::error_id());
            }
        }
    }
    */

    // We consume `P`, assuming that it is either cheap to clone or just a reference to some static
    // data (e.g. Linux dentry structs in the kernel). Ownership makes it easier to hold parents
    // of `path` in the same variable.
    fn node_id_for_path(&mut self, path: P) -> Option<NodeId> {
        // Make a separate block holding `NameContext` on the stack in the hope that it's easier
        // for the compiler to know that `NameContext` and `NodeContext` don't need to live
        // simultaneously.
        let (name_ids, depth, root_node_id) = {
            let mut ctx = NameContext {
                path,
                name_ids: unsafe { &mut *self.string_id_buffer() },
                depth: 0,
                name_id_context: self.name_id_context(),
            };

            // traverse up to the root path and record string ids for each component.
            repeat(MAX_PATH_COMPONENTS as _, obtain_name_ids_inner, &mut ctx);

            // `path` represents the root node now, as it does not have a parent. Get an ID
            let root_node_id = match self.root_node_id(&ctx.path) {
                Some(node) => node,
                None => return None,
            };
            let NameContext {
                depth, name_ids, ..
            } = ctx;
            (name_ids, depth, root_node_id)
        };
        let mut ctx = NodeContext {
            name_ids,
            depth,
            node_id: root_node_id,
            cache: self,
            phantom1: PhantomData::default(),
            phantom2: PhantomData::default(),
        };

        // now iterate back from the root to the leaf:
        repeat(ctx.depth as _, obtain_node_ids_inner, &mut ctx);

        // `node_id` is now the id that represents the original `path`.
        Some(ctx.node_id)
    }

    fn enumerate_path(&self, mut node_id: NodeId, mut closure: impl FnMut(StringId)) {
        loop {
            let node = match self.node_for_id(node_id) {
                Some(node) => node,
                None => break,
            };
            closure(node.name_id);
            match node.parent_id {
                Some(parent) => node_id = parent,
                None => break,
            }
        }
    }
}

struct NameContext<'a, P: PathRep<S>, S> {
    pub path: P,
    pub name_ids: &'a mut [StringId; MAX_PATH_COMPONENTS],
    pub depth: usize,
    pub name_id_context: *mut S,
}

extern "C" fn obtain_name_ids_inner<'a, P: PathRep<S>, S>(
    _index: u64,
    ctx: &mut NameContext<'a, P, S>,
) -> LoopReturn {
    let parent = match ctx.path.parent() {
        Some(parent) => parent,
        None => return LoopReturn::LoopBreak,
    };
    if ctx.depth >= MAX_PATH_COMPONENTS {
        // Does not happen due to the number of iterations we have set, but the compiler does
        // not know. It therefore inserts a bounds check which can panic, which is not allowed
        // by the eBPF verifier. We therefore do the bounds check manually.
        return LoopReturn::LoopBreak;
    }
    ctx.name_ids[ctx.depth] = ctx.path.name_id(ctx.name_id_context);
    ctx.depth += 1;
    ctx.path = parent;
    LoopReturn::LoopContinue
}

struct NodeContext<'a, C: NodeCacheTrait<P, S>, P: PathRep<S>, S> {
    pub cache: &'a mut C,
    pub name_ids: &'a mut [StringId; MAX_PATH_COMPONENTS],
    pub depth: usize,
    pub node_id: NodeId,
    phantom1: PhantomData<P>, // use `P` to make the compiler happy
    phantom2: PhantomData<S>, // use `P` to make the compiler happy
}

extern "C" fn obtain_node_ids_inner<'a, C: NodeCacheTrait<P, S>, P: PathRep<S>, S>(
    _index: u64,
    ctx: &mut NodeContext<'a, C, P, S>,
) -> LoopReturn {
    ctx.depth -= 1;
    if ctx.depth >= MAX_PATH_COMPONENTS {
        // Does not happen due to the number of iterations we have set, but the compiler does
        // not know. It therefore inserts a bounds check which can panic, which is not allowed
        // by the eBPF verifier. We therefore do the bounds check manually.
        return LoopReturn::LoopBreak;
    }
    let node = PathNode {
        parent_id: Some(ctx.node_id),
        name_id: ctx.name_ids[ctx.depth],
    };
    ctx.node_id = ctx.cache.id_for_node(&node).unwrap_or_else(|| {
        let new_id = ctx.cache.new_id();
        if ctx.cache.insert_node(&node, new_id) {
            ctx.cache.consume_id();
            new_id
        } else {
            // Another thread beat us – look it up again.
            ctx.cache.id_for_node(&node).unwrap_or(NodeId::error_id())
        }
    });
    LoopReturn::LoopContinue
}

#[cfg(feature = "user")]
mod pods {
    use aya::Pod;
    use super::*;

    unsafe impl Pod for NodeId {}
    unsafe impl Pod for PathNode {}
}

#[cfg(test)]
mod tests {

    use crate::{
        mock_node_cache::{MockNodeCache, MockPath},
        mock_strings_cache::MockStringsCache,
        node_cache::NodeCacheTrait,
    };
    use std::{cell::RefCell, collections::HashSet};

    static TEST_PATHS: &[&str] = &[
        "/a/b/c/d/e/f/g",
        "/a/b/c/g",
        "/a/b/c/d/e/f",
        "/a/b/c/d/f/e",
        "/a/b/c/e/d/f",
        "/a/b/c/e/f/d",
        "/a/b/c/f/d/e",
        "/a/b/c/f/e/d",
        "/a/b/d/c/e/f",
        "/a/b/d/c/f/e",
        "/a/b/d/e/c/f",
        "/a/b/d/e/f/c",
        "/a/b/d/f/c/e",
        "/a/b/d/f/e/c",
        "/a/b/e/c/d/f",
        "/a/b/e/c/f/d",
        "/a/b/e/d/c/f",
        "/a/b/e/d/f/c",
        "/a/b/e/f/c/d",
        "/a/b/e/f/d/c",
        "/a/c/b/d/e/f",
        "/a/c/b/d/f/e",
        "/x",
        "/",
        concat!(
            "/L0/L1/L2/L3/L4/L5/L6/L7/L8/L9/L10/L11/L12/L13/L14/L15/L16/L17/L18/L19/L20",
            "/L21/L22/L23/L24/L25/L26/L27/L28/L29/L30/L31/L32/L33/L34/L35/L36/L37/L38/L39",
            "/L40/L41/L42/L43/L44/L45/L46/L47/L48/L49/L50/L51/L52/L53/L54/L55/L56/L57/L58",
            "/L59/L60/L61/L62/L63/L64/L65/L66/L67/L68/L69/L70/L71/L72/L73/L74/L75/L76/L77",
            "/L78/L79/L80/L81/L82/L83/L84/L85/L86/L87/L88/L89/L90/L91/L92/L93/L94/L95/L96",
            "/L97/L98/L99/L100/L101/L102/L103/L104/L105/L106/L107/L108/L109/L110/L111/L112",
            "/L113/L114/L115/L116/L117/L118/L119/L120/L121/L122/L123/L124/L125/L126/L127",
            "/L128/L129/L130/L131/L132/L133/L134/L135/L136/L137/L138/L139/L140/L141/L142",
            "/L143/L144/L145/L146/L147/L148/L149/L150/L151/L152/L153/L154/L155/L156/L157",
            "/L158/L159/L160/L161/L162/L163/L164/L165/L166/L167/L168/L169/L170/L171/L172",
            "/L173/L174/L175/L176/L177/L178/L179/L180/L181/L182/L183/L184/L185/L186/L187",
            "/L188/L189/L190/L191/L192/L193/L194/L195/L196/L197/L198/L199/L200/L201/L202",
            "/L203/L204/L205/L206/L207/L208/L209/L210/L211/L212/L213/L214/L215/L216/L217",
            "/L218/L219/L220/L221/L222/L223/L224/L225/L226/L227/L228/L229/L230/L231/L232",
            "/L233/L234/L235/L236/L237/L238/L239/L240/L241/L242/L243/L244/L245/L246/L247",
            "/L248/L249/L250/L251/L252/L253/L254/L255"
        ),
    ];

    #[test]
    fn test_node_cache() {
        let strings_cache = RefCell::new(MockStringsCache::new());
        let mut node_cache = MockNodeCache::new(&strings_cache);
        let mock_paths: Vec<_> = TEST_PATHS
            .iter()
            .map(|&p| MockPath::new(p, &strings_cache))
            .collect();
        let node_ids: Vec<_> = mock_paths
            .into_iter()
            .filter_map(|p| node_cache.node_id_for_path(p))
            .collect();
        let hashed_ids: HashSet<_> = node_ids.iter().cloned().collect();
        assert!(hashed_ids.len() == node_ids.len()); // check whether there were dudplicates
        for (index, &id) in node_ids.iter().enumerate() {
            let mut path_vec = Vec::<String>::new();
            node_cache.enumerate_path(id, |string_id| {
                path_vec.push(strings_cache.borrow().string_for_identifier(string_id));
            });
            path_vec.reverse();
            let path = path_vec.join("/");
            println!("reconstructed path = {}", path);
            assert!(path == TEST_PATHS[index]);
        }
    }
}

//! The frame tree: interned names and the edges between them.

use super::history::History;
use alloc::{boxed::Box, vec::Vec};
use core::fmt;
use hashbrown::HashMap;

/// A frame's position in [`Frames`]. Private to the registry: no handle
/// leaves it, so an id is only ever used while the frame it names is live.
pub(super) type FrameId = usize;

/// One known frame.
struct Frame {
    name: Box<str>,
    /// The edge to the frame's parent; `None` for a root.
    edge: Option<Edge>,
    /// How many edges name this frame as their parent.
    children: usize,
}

/// A child frame's edge to its parent, with the history stored on it.
#[derive(Debug)]
pub(super) struct Edge {
    pub(super) parent: FrameId,
    pub(super) history: History,
}

/// The frames a registry knows, and the tree their edges form.
///
/// A name is known exactly while a stored edge names it, as child or as
/// parent. That is one rule kept by one invariant: a slot is occupied iff
/// its frame has an edge or at least one child, and the name index holds
/// exactly the occupied slots. Every mutation below restores it before it
/// returns, so a name leaves the index in the same call that removes its
/// last edge reference.
///
/// Slots are reused through a free list and never renumbered while
/// occupied, so the ids stored in edges stay valid.
#[derive(Default)]
pub(super) struct Frames {
    slots: Vec<Option<Frame>>,
    free: Vec<FrameId>,
    index: HashMap<Box<str>, FrameId>,
}

impl Frames {
    /// The id of a known frame. The empty name is never known: no edge can
    /// name it.
    #[inline]
    pub(super) fn id(
        &self,
        name: &str,
    ) -> Option<FrameId> {
        self.index.get(name).copied()
    }

    #[inline]
    fn frame(
        &self,
        id: FrameId,
    ) -> Option<&Frame> {
        self.slots.get(id)?.as_ref()
    }

    #[inline]
    fn frame_mut(
        &mut self,
        id: FrameId,
    ) -> Option<&mut Frame> {
        self.slots.get_mut(id)?.as_mut()
    }

    /// The name of a live frame. The empty string for an id that names no
    /// frame, which no id held by an edge or returned by [`Self::id`] is.
    pub(super) fn name(
        &self,
        id: FrameId,
    ) -> &str {
        self.frame(id).map_or("", |frame| &frame.name)
    }

    /// The edge from `id` to its parent; `None` for a root.
    #[inline]
    pub(super) fn edge(
        &self,
        id: FrameId,
    ) -> Option<&Edge> {
        self.frame(id)?.edge.as_ref()
    }

    /// The stored history on the edge from `id` to its parent.
    #[inline]
    pub(super) fn history_mut(
        &mut self,
        id: FrameId,
    ) -> Option<&mut History> {
        self.frame_mut(id)?
            .edge
            .as_mut()
            .map(|edge| &mut edge.history)
    }

    /// Every stored history, in no particular order.
    pub(super) fn histories_mut(&mut self) -> impl Iterator<Item = &mut History> {
        self.slots
            .iter_mut()
            .flatten()
            .filter_map(|frame| frame.edge.as_mut())
            .map(|edge| &mut edge.history)
    }

    /// How many edges lie between `id` and its tree's root, and that root.
    ///
    /// The walk ends because the tree is acyclic: every edge passed a cycle
    /// check before it was attached, and no other mutation adds one.
    #[inline]
    pub(super) fn ascent(
        &self,
        mut id: FrameId,
    ) -> (usize, FrameId) {
        let mut depth = 0;
        while let Some(edge) = self.edge(id) {
            id = edge.parent;
            depth += 1;
        }
        (depth, id)
    }

    /// The lowest frame that is `a`, `b`, or an ancestor of both; `None` if
    /// the two live in different trees.
    #[inline]
    pub(super) fn meeting_point(
        &self,
        mut a: FrameId,
        mut b: FrameId,
    ) -> Option<FrameId> {
        let (mut depth_a, root_a) = self.ascent(a);
        let (mut depth_b, root_b) = self.ascent(b);
        if root_a != root_b {
            return None;
        }
        // Lift the deeper side level with the other, then both together.
        // A missing edge cannot be met below a shared root; if it ever
        // were, the walks stop where they stand instead of looping.
        while depth_a > depth_b {
            a = self.edge(a)?.parent;
            depth_a -= 1;
        }
        while depth_b > depth_a {
            b = self.edge(b)?.parent;
            depth_b -= 1;
        }
        while a != b {
            a = self.edge(a)?.parent;
            b = self.edge(b)?.parent;
        }
        Some(a)
    }

    /// Whether `ancestor` is `id` or lies above it.
    pub(super) fn is_at_or_above(
        &self,
        ancestor: FrameId,
        mut id: FrameId,
    ) -> bool {
        loop {
            if id == ancestor {
                return true;
            }
            match self.edge(id) {
                Some(edge) => id = edge.parent,
                None => return false,
            }
        }
    }

    /// The id of `name`, giving it a slot if it has none.
    ///
    /// A fresh slot has neither an edge nor a child, which the invariant
    /// does not allow to last: the caller attaches an edge to it or below
    /// it before returning.
    fn intern(
        &mut self,
        name: &str,
    ) -> FrameId {
        if let Some(id) = self.id(name) {
            return id;
        }
        let frame = Frame {
            name: name.into(),
            edge: None,
            children: 0,
        };
        let vacant = self
            .free
            .pop()
            .and_then(|id| Some((id, self.slots.get_mut(id)?)));
        let id = if let Some((id, slot)) = vacant {
            *slot = Some(frame);
            id
        } else {
            self.slots.push(Some(frame));
            self.slots.len() - 1
        };
        self.index.insert(name.into(), id);
        id
    }

    /// Frees the slot of a frame that no edge names any more. A frame that
    /// still has an edge or a child keeps its slot.
    fn release(
        &mut self,
        id: FrameId,
    ) {
        let unreferenced = self
            .frame(id)
            .is_some_and(|frame| frame.edge.is_none() && frame.children == 0);
        if !unreferenced {
            return;
        }
        if let Some(frame) = self.slots.get_mut(id).and_then(Option::take) {
            self.index.remove(&*frame.name);
            self.free.push(id);
        }
    }

    /// Puts the edge `parent <- child` into the tree, making both names
    /// known. Returns the child's id.
    ///
    /// The caller has checked that `child` has no edge and that the new
    /// edge closes no cycle.
    pub(super) fn attach(
        &mut self,
        child: &str,
        parent: &str,
        history: History,
    ) -> FrameId {
        let parent = self.intern(parent);
        let child = self.intern(child);
        if let Some(frame) = self.frame_mut(parent) {
            frame.children += 1;
        }
        if let Some(frame) = self.frame_mut(child) {
            frame.edge = Some(Edge { parent, history });
        }
        child
    }

    /// Replaces the edge of `child` by one to `parent`, making that name
    /// known, and drops the history the old edge held.
    ///
    /// The old parent stops being known if no other edge names it. The
    /// caller has checked that `child` has an edge, that `parent` is not
    /// its current parent, and that the new edge closes no cycle.
    pub(super) fn reattach(
        &mut self,
        child: FrameId,
        parent: &str,
        history: History,
    ) {
        let parent = self.intern(parent);
        if let Some(frame) = self.frame_mut(parent) {
            frame.children += 1;
        }
        let replaced = self
            .frame_mut(child)
            .and_then(|frame| frame.edge.replace(Edge { parent, history }));
        if let Some(replaced) = replaced {
            if let Some(frame) = self.frame_mut(replaced.parent) {
                frame.children = frame.children.saturating_sub(1);
            }
            self.release(replaced.parent);
        }
    }

    /// Removes the edge from `child` to its parent, with its history.
    /// Returns whether there was one.
    ///
    /// A frame left without any edge reference stops being known: the
    /// former parent if no other edge names it, and the child unless it has
    /// children of its own.
    pub(super) fn detach(
        &mut self,
        child: FrameId,
    ) -> bool {
        let Some(edge) = self.frame_mut(child).and_then(|frame| frame.edge.take()) else {
            return false;
        };
        if let Some(parent) = self.frame_mut(edge.parent) {
            parent.children = parent.children.saturating_sub(1);
        }
        self.release(edge.parent);
        self.release(child);
        true
    }
}

/// Prints each known frame with its parent and a summary of its history;
/// see [`History`]'s `Debug`.
impl fmt::Debug for Frames {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let mut frames = f.debug_map();
        for frame in self.slots.iter().flatten() {
            match &frame.edge {
                Some(edge) => {
                    frames.entry(&frame.name, &(self.name(edge.parent), &edge.history));
                }
                None => {
                    frames.entry(&frame.name, &"root");
                }
            }
        }
        frames.finish()
    }
}

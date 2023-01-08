//! Realizing a virtual dom on the actual DOM
//!
//! A bundle, borrowed from the mathematical meaning, is any structure over some base space.
//! In our case, the base space is the virtual dom we're trying to render.
//! In order to efficiently implement updates, and diffing, additional information has to be
//! kept around. This information is carried in the bundle.

use web_sys::Element;

use crate::html::{AnyScope, NodeRef};
use crate::virtual_dom::VNode;

mod bcomp;
mod blist;
mod bnode;
mod bportal;
mod braw;
mod bsuspense;
mod btag;
mod btext;
mod subtree_root;

mod traits;
mod utils;

use bcomp::BComp;
use blist::BList;
use bnode::BNode;
use bportal::BPortal;
use braw::BRaw;
use bsuspense::BSuspense;
use btag::{BTag, Registry};
use btext::BText;
pub use subtree_root::set_event_bubbling;
pub(crate) use subtree_root::BSubtree;
use subtree_root::EventDescriptor;
use traits::{Reconcilable, ReconcileTarget};
use utils::{insert_node, test_log};

/// A Bundle Primitives
#[derive(Debug, Clone)]
pub(crate) struct BundleLocation {
    pub internal_ref: NodeRef,

    pub parent: Element,
    pub next_sibling: NodeRef,

    root: BSubtree,
}

impl BundleLocation {
    /// Creates a bundle location with no parent bundle location.
    ///
    /// This is used for renderer to create a bundle location for the top most component.
    pub fn new(host_element: Element) -> Self {
        let root = BSubtree::create_root(&host_element);

        BundleLocation {
            internal_ref: NodeRef::default(),
            parent: host_element,
            next_sibling: NodeRef::default(),
            root,
        }
    }

    /// Creates a bundle location with from a parent.
    fn new_child(root: &BSubtree, parent_element: Element, next_sibling: NodeRef) -> Self {
        let stable_next_sibling = NodeRef::default();
        stable_next_sibling.link(next_sibling);

        BundleLocation {
            internal_ref: NodeRef::default(),
            parent: parent_element,
            next_sibling: stable_next_sibling,
            root: root.clone(),
        }
    }
}

/// A Bundle.
///
/// Each component holds a bundle that represents a realised layout, designated by a [VNode].
///
/// This is not to be confused with [BComp], which represents a component in the position of a
/// bundle layout.
#[derive(Debug)]
pub(crate) struct Bundle {
    layout: BNode,
    pub location: BundleLocation,
}

impl Bundle {
    /// Creates a new bundle.
    pub fn new(location: BundleLocation) -> Self {
        Self {
            layout: BNode::List(BList::new()),
            location,
        }
    }

    /// Shifts the bundle into a different position.
    pub fn shift(&mut self, new_parent: Element, new_next_sibling: NodeRef) {
        self.location.parent = new_parent;
        self.location.next_sibling.link(new_next_sibling);

        self.layout
            .shift(&self.location.parent, self.location.next_sibling.clone());
    }

    /// Applies a virtual dom layout to current bundle.
    pub fn reconcile(&mut self, parent_scope: &AnyScope, next_layout: VNode) {
        #[cfg(feature = "hydration")]
        self.location.next_sibling.debug_assert_not_trapped();

        let next_ref = next_layout.reconcile_node(
            &self.location.root,
            parent_scope,
            &self.location.parent,
            self.location.next_sibling.clone(),
            &mut self.layout,
        );

        self.location.internal_ref.link(next_ref);
    }

    /// Detaches current bundle.
    pub fn detach(self, parent_to_detach: bool) {
        self.layout
            .detach(&self.location.root, &self.location.parent, parent_to_detach);
        self.location.internal_ref.set(None);
    }
}

#[cfg(feature = "hydration")]
#[path = "."]
mod feat_hydration {
    pub(super) use super::traits::Hydratable;
    pub(super) use super::utils::node_type_str;
    #[path = "./fragment.rs"]
    mod fragment;
    pub(crate) use fragment::Fragment;

    use super::*;
    impl Bundle {
        /// Creates a bundle by hydrating a virtual dom layout.
        pub fn hydrate(
            parent_scope: &AnyScope,
            location: BundleLocation,
            fragment: &mut Fragment,
            layout: VNode,
        ) -> Self {
            let (node_ref, bundle) =
                layout.hydrate(&location.root, parent_scope, &location.parent, fragment);
            location.internal_ref.link(node_ref);

            Self {
                location,
                layout: bundle,
            }
        }
    }
}
#[cfg(feature = "hydration")]
pub(crate) use feat_hydration::*;

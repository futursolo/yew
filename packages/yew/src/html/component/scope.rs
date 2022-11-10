//! Component scope module

use std::any::TypeId;
#[cfg(feature = "csr")]
use std::cell::RefCell;
use std::rc::Rc;
use std::{fmt, iter};

#[cfg(feature = "csr")]
use super::lifecycle::ComponentState;
use super::Component;
use crate::callback::Callback;
use crate::context::{ContextHandle, ContextProvider, ContextStore};
#[cfg(all(feature = "hydration", feature = "ssr"))]
use crate::html::RenderMode;

struct ScopeInner {
    id: usize,
    type_id: TypeId,

    #[cfg(feature = "csr")]
    pub(crate) state: RefCell<Option<ComponentState>>,

    parent: Option<Scope>,
}

/// Untyped scope used for accessing parent scope
#[derive(Clone)]
pub struct Scope {
    inner: Rc<ScopeInner>,
}

impl fmt::Debug for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AnyScope<_>")
    }
}

impl Scope {
    pub(crate) fn id(&self) -> usize {
        self.inner.id
    }

    /// Schedules a render.
    pub(crate) fn schedule_render(&self) {
        #[cfg(feature = "csr")]
        {
            use crate::scheduler;

            let scope = self.clone();
            scheduler::push(move || ComponentState::run_render(&scope));
        }
    }

    /// Returns the parent scope
    pub fn parent(&self) -> Option<&Scope> {
        self.inner.parent.as_ref()
    }

    /// Returns the type of the linked component
    pub fn type_id(&self) -> TypeId {
        self.inner.type_id
    }

    /// Attempts checks the component type of current scope
    ///
    /// Returns [`None`] if the self value can't be cast into the target type.
    pub(crate) fn is_scope_of<COMP: Component>(&self) -> bool {
        self.type_id() == TypeId::of::<COMP>()
    }

    /// Attempts to find a parent scope of a certain type
    ///
    /// Returns [`None`] if no parent scope with the specified type was found.
    pub(crate) fn find_parent_scope<COMP: Component>(&self) -> Option<&Scope> {
        iter::successors(Some(self), |scope| scope.parent()).find(|m| m.is_scope_of::<COMP>())
    }

    /// Accesses a value provided by a parent `ContextProvider` component of the
    /// same type.
    pub fn context<T: Clone + PartialEq + 'static>(
        &self,
        callback: Callback<T>,
    ) -> Option<(T, ContextHandle<T>)> {
        let scope = self.find_parent_scope::<ContextProvider<T>>()?;
        let store = ContextStore::<T>::get(scope)?;
        Some(ContextStore::subscribe_consumer(store, callback))
    }
}

#[cfg(feature = "ssr")]
mod feat_ssr {
    use std::fmt::Write;

    use super::*;
    use crate::functional::HookContext;
    #[cfg(feature = "hydration")]
    use crate::html::RenderMode;
    use crate::html::{Intrinsical, RenderError};
    use crate::platform::fmt::BufWriter;

    impl Scope {
        pub(crate) async fn render_into_stream<'a>(
            &'a self,
            mountable: Rc<dyn Intrinsical>,
            w: &'a mut BufWriter,
            hydratable: bool,
        ) {
            // Rust's Future implementation is stack-allocated and incurs zero runtime-cost.
            //
            // If the content of this channel is ready before it is awaited, it is
            // similar to taking the value from a mutex lock.

            let mut ctx = HookContext::new(
                self.clone(),
                #[cfg(feature = "hydration")]
                RenderMode::Ssr,
                #[cfg(feature = "hydration")]
                None,
            );
            let collectable = mountable.create_collectable();

            if hydratable {
                collectable.write_open_tag(w);
            }

            let html = loop {
                match mountable.render(&mut ctx) {
                    Ok(m) => break m,
                    Err(RenderError::Suspended(e)) => e.await,
                }
            };

            html.render_into_stream(w, self, hydratable).await;

            if let Some(prepared_state) = ctx.prepare_state() {
                let _ = w.write_str(r#"<script type="application/x-yew-comp-state">"#);
                let _ = w.write_str(&prepared_state);
                let _ = w.write_str(r#"</script>"#);
            }

            if hydratable {
                collectable.write_close_tag(w);
            }
        }
    }
}

#[cfg(any(feature = "ssr", feature = "csr"))]
mod feat_csr_ssr {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::html::Intrinsical;

    static COMP_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

    impl Scope {
        /// Crate a scope with an optional parent scope
        pub(crate) fn new(mountable: &dyn Intrinsical, parent: Option<Scope>) -> Self {
            Scope {
                inner: Rc::new(ScopeInner {
                    type_id: mountable.type_id(),

                    #[cfg(feature = "csr")]
                    state: RefCell::new(None),
                    parent,

                    id: COMP_ID_COUNTER.fetch_add(1, Ordering::SeqCst),
                }),
            }
        }
    }
}

#[cfg(feature = "csr")]
mod feat_csr {
    use web_sys::Element;

    use super::*;
    use crate::dom_bundle::{BSubtree, DomSlot};
    use crate::html::{Intrinsical, NodeRef};
    use crate::HookContext;

    impl Scope {
        #[cfg(test)]
        pub(crate) fn test() -> Self {
            Self {
                inner: Rc::new(ScopeInner {
                    id: 0,
                    type_id: TypeId::of::<()>(),
                    state: RefCell::default(),
                    parent: None,
                }),
            }
        }

        pub(crate) fn state_cell(&self) -> &RefCell<Option<ComponentState>> {
            &self.inner.state
        }

        pub(crate) fn reuse(&self, mountable: Rc<dyn Intrinsical>, next_sibling: NodeRef) {
            ComponentState::run_update(self, Some(mountable), Some(next_sibling));
        }

        /// Mounts a component with `props` to the specified `element` in the DOM.
        pub(crate) fn mount(
            &self,
            mountable: Rc<dyn Intrinsical>,
            root: BSubtree,
            parent: Element,
            next_sibling: NodeRef,
            internal_ref: NodeRef,
        ) {
            internal_ref.link(next_sibling.clone());
            let stable_next_sibling = NodeRef::default();
            stable_next_sibling.link(next_sibling);

            let slot = DomSlot::builder()
                .root(root)
                .parent(parent)
                .next_sibling(stable_next_sibling)
                .build();

            let ctx = HookContext::new(
                self.clone(),
                #[cfg(all(feature = "hydration", feature = "ssr"))]
                RenderMode::Render,
                #[cfg(feature = "hydration")]
                None,
            );

            ComponentState::run_create(ctx, self.clone(), mountable, slot);
        }

        /// Process an event to destroy a component
        pub(crate) fn destroy(self, parent_to_detach: bool) {
            ComponentState::run_destroy(&self, parent_to_detach);
        }

        pub(crate) fn shift_node(&self, parent: Element, next_sibling: NodeRef) {
            ComponentState::run_shift(self, parent, next_sibling);
        }
    }
}

#[cfg(feature = "hydration")]
mod feat_hydration {
    use wasm_bindgen::JsCast;
    use web_sys::{Element, HtmlScriptElement};

    use super::*;
    use crate::dom_bundle::{BSubtree, DomSlot, Fragment, Realized};
    use crate::html::{Intrinsical, NodeRef};
    use crate::HookContext;

    impl Scope {
        /// Hydrates the component.
        ///
        /// Returns a pending NodeRef of the next sibling.
        ///
        /// # Note
        ///
        /// This method is expected to collect all the elements belongs to the current component
        /// immediately.
        pub(crate) fn hydrate(
            &self,
            mountable: Rc<dyn Intrinsical>,
            root: BSubtree,
            parent: Element,
            fragment: &mut Fragment,
            internal_ref: NodeRef,
        ) {
            let collectable = mountable.create_collectable();
            let mut fragment = Fragment::collect_between(fragment, &collectable, &parent);

            let prepared_state = match fragment
                .back()
                .cloned()
                .and_then(|m| m.dyn_into::<HtmlScriptElement>().ok())
            {
                Some(m) if m.type_() == "application/x-yew-comp-state" => {
                    fragment.pop_back();
                    parent.remove_child(&m).unwrap();
                    Some(m.text().unwrap())
                }
                _ => None,
            };

            let slot = DomSlot::builder()
                .content(Realized::Fragement(fragment))
                .root(root)
                .parent(parent)
                .internal_ref(internal_ref)
                .build();

            let ctx = HookContext::new(
                self.clone(),
                #[cfg(feature = "ssr")]
                RenderMode::Hydration,
                prepared_state.as_deref(),
            );
            ComponentState::run_create(ctx, self.clone(), mountable, slot);
        }
    }
}

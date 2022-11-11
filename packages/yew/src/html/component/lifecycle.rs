//! Component lifecycle module

use std::rc::Rc;

use web_sys::Element;

use super::scope::Scope;
use crate::dom_bundle::{DomSlot, Realized};
use crate::html::{Html, Intrinsical, NodeRef, RenderError};
use crate::suspense::{resume_suspension, suspend_suspension, DispatchSuspension, Suspension};
use crate::{Callback, ContextProvider, HookContext};

pub(crate) struct ComponentState {
    pub(super) ctx: HookContext,
    intrinsic: Rc<dyn Intrinsical>,
    pub slot: DomSlot,

    #[cfg(feature = "hydration")]
    pending_intrinsic: Option<Rc<dyn Intrinsical>>,
    suspension: Option<Suspension>,
}

impl ComponentState {
    #[tracing::instrument(
        level = tracing::Level::DEBUG,
        name = "create",
        skip_all,
        fields(component.id = _scope.id()),
    )]
    fn new(
        _scope: &Scope,
        ctx: HookContext,
        intrinsic: Rc<dyn Intrinsical>,
        slot: DomSlot,
    ) -> Self {
        Self {
            ctx,
            intrinsic,

            suspension: None,

            slot,

            #[cfg(feature = "hydration")]
            pending_intrinsic: None,
        }
    }

    pub fn run_create(
        scope: &Scope,
        ctx: HookContext,
        intrinsic: Rc<dyn Intrinsical>,
        slot: DomSlot,
    ) {
        let mut current_state = scope.state_cell().borrow_mut();

        if current_state.is_none() {
            let mut self_ = Self::new(scope, ctx, intrinsic, slot);
            self_.render(scope);

            // We are safe to assign afterwards as we mutably borrow the state and don't release it
            // until this function returns.
            *current_state = Some(self_);
        }
    }

    pub fn run_render(scope: &Scope, step: usize) {
        let current_step = scope.render_step_cell().get();
        // The desired change has been applied.
        if current_step > step {
            return;
        }

        scope.render_step_cell().set(current_step + 1);
        if let Some(state) = scope.state_cell().borrow_mut().as_mut() {
            state.render(scope);
        }
    }

    pub fn run_shift(scope: &Scope, next_parent: Element, next_sibling: NodeRef) {
        if let Some(state) = scope.state_cell().borrow_mut().as_mut() {
            state.shift(next_parent, next_sibling);
        }
    }

    pub fn run_update(
        scope: &Scope,
        intrinsic: Option<Rc<dyn Intrinsical>>,
        next_sibling: Option<NodeRef>,
    ) {
        if let Some(state) = scope.state_cell().borrow_mut().as_mut() {
            state.changed(scope, intrinsic, next_sibling);
        }
    }

    pub fn run_destroy(scope: &Scope, parent_to_detach: bool) {
        if let Some(state) = scope.state_cell().borrow_mut().take() {
            state.destroy(scope, parent_to_detach);
        }
    }

    fn resume_existing_suspension(&mut self, scope: &Scope) {
        if let Some(m) = self.suspension.take() {
            let suspense_scope = scope
                .find_parent_scope::<ContextProvider<DispatchSuspension>>()
                .unwrap();
            resume_suspension(suspense_scope, m);
        }
    }

    pub fn shift(&mut self, next_parent: Element, next_next_sibling: NodeRef) {
        match self.slot.content {
            Realized::Bundle(ref mut bundle) => {
                bundle.shift(&next_parent, next_next_sibling.clone());
            }
            #[cfg(feature = "hydration")]
            Realized::Fragement(ref mut fragment) => {
                fragment.shift(&next_parent, next_next_sibling.clone());
            }
        }

        self.slot.parent = next_parent;
        self.slot.next_sibling.link(next_next_sibling);
    }

    #[tracing::instrument(
        level = tracing::Level::DEBUG,
        skip(self),
        fields(component.id = scope.id())
    )]
    fn destroy(mut self, scope: &Scope, parent_to_detach: bool) {
        self.ctx.destroy();
        self.resume_existing_suspension(scope);

        match self.slot.content {
            Realized::Bundle(bundle) => {
                bundle.detach(&self.slot.root, &self.slot.parent, parent_to_detach);
            }
            // We need to detach the hydrate fragment if the component is not hydrated.
            #[cfg(feature = "hydration")]
            Realized::Fragement(fragment) => {
                fragment.detach(&self.slot.root, &self.slot.parent, parent_to_detach);
            }
        }

        self.slot.internal_ref.set(None);
    }

    #[tracing::instrument(
        level = tracing::Level::DEBUG,
        skip_all,
        fields(component.id = scope.id())
    )]
    fn render(&mut self, scope: &Scope) {
        match self.intrinsic.render(&mut self.ctx) {
            Ok(vnode) => self.commit_render(scope, vnode),
            Err(RenderError::Suspended(susp)) => self.suspend(scope, susp),
        };
    }

    fn suspend(&mut self, scope: &Scope, suspension: Suspension) {
        // Currently suspended, we re-use previous root node and send
        // suspension to parent element.

        if suspension.resumed() {
            self.render(scope);
        } else {
            // We schedule a render after current suspension is resumed.
            let suspense_scope = scope
                .find_parent_scope::<ContextProvider<DispatchSuspension>>()
                .expect("To suspend rendering, a <Suspense /> component is required.");

            {
                let scope = scope.clone();
                suspension.listen(Callback::from(move |_| {
                    scope.schedule_render();
                }));
            }

            if let Some(ref last_suspension) = self.suspension {
                if &suspension != last_suspension {
                    // We remove previous suspension from the suspense.
                    resume_suspension(suspense_scope, last_suspension.clone())
                }
            }
            self.suspension = Some(suspension.clone());

            suspend_suspension(suspense_scope, suspension);
        }
    }

    fn commit_render(&mut self, scope: &Scope, new_root: Html) {
        // Currently not suspended, we remove any previous suspension and update
        // normally.
        self.resume_existing_suspension(scope);

        match self.slot.content {
            Realized::Bundle(ref mut bundle) => {
                let new_node_ref = bundle.reconcile(
                    &self.slot.root,
                    scope,
                    &self.slot.parent,
                    self.slot.next_sibling.clone(),
                    new_root,
                );
                self.slot.internal_ref.link(new_node_ref);

                let has_pending_props = self.rendered(scope);
                if has_pending_props {
                    self.changed(scope, None, None);
                }
            }

            #[cfg(feature = "hydration")]
            Realized::Fragement(ref mut fragment) => {
                use crate::dom_bundle::Bundle;

                let (node, bundle) = Bundle::hydrate(
                    &self.slot.root,
                    scope,
                    &self.slot.parent,
                    fragment,
                    new_root,
                );

                // We trim all text nodes before checking as it's likely these are whitespaces.
                fragment.trim_start_text_nodes(&self.slot.parent);

                assert!(fragment.is_empty(), "expected end of component, found node");

                self.slot.internal_ref.link(node);

                self.slot.content = Realized::Bundle(bundle);
            }
        };
    }

    #[tracing::instrument(
        level = tracing::Level::DEBUG,
        skip(self, intrinsic),
        fields(component.id = scope.id())
    )]
    pub(super) fn changed(
        &mut self,
        scope: &Scope,
        intrinsic: Option<Rc<dyn Intrinsical>>,
        next_sibling: Option<NodeRef>,
    ) {
        if let Some(next_sibling) = next_sibling {
            // When components are updated, their siblings were likely also updated
            // We also need to shift the bundle so next sibling will be synced to child
            // components.
            self.slot.next_sibling.link(next_sibling);
        }

        // Only trigger changed if props were changed / next sibling has changed.
        let schedule_render = {
            #[cfg(feature = "hydration")]
            {
                if let Some(intrinsic) = intrinsic.or_else(|| self.pending_intrinsic.take()) {
                    match self.slot.content {
                        Realized::Bundle { .. } => {
                            self.pending_intrinsic = None;
                            if !self.intrinsic.intrinsic_eq(intrinsic.as_ref()) {
                                self.intrinsic = intrinsic;
                            }
                            true
                        }
                        Realized::Fragement { .. } => {
                            self.pending_intrinsic = Some(intrinsic);
                            false
                        }
                    }
                } else {
                    false
                }
            }

            #[cfg(not(feature = "hydration"))]
            {
                intrinsic
                    .and_then(|m| (!self.intrinsic.intrinsic_eq(m.as_ref())).then_some(m))
                    .map(|m| {
                        self.intrinsic = m;
                        true
                    })
                    .unwrap_or(false)
            }
        };

        tracing::trace!("props_update(schedule_render={})", schedule_render);

        if schedule_render {
            self.render(scope)
        }
    }

    #[tracing::instrument(
        level = tracing::Level::DEBUG,
        skip(self),
        fields(component.id = _scope.id())
    )]
    pub(super) fn rendered(&mut self, _scope: &Scope) -> bool {
        if self.suspension.is_none() {
            self.ctx.rendered();
        }

        #[cfg(feature = "hydration")]
        {
            self.pending_intrinsic.is_some()
        }
        #[cfg(not(feature = "hydration"))]
        {
            false
        }
    }
}

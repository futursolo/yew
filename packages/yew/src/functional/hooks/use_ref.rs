use std::cell::{BorrowError, BorrowMutError, RefCell};
use std::rc::Rc;

use crate::functional::{hook, use_memo};
use crate::html::IS_RENDERING;
use std::fmt;
use std::sync::atomic::Ordering;

use crate::NodeRef;

/// State handle for [`use_ref`].
///
/// Compare to [RefCell], from the standard library, this handle is render-time safe.
pub struct UseRefHandle<T>
where
    T: 'static,
{
    inner: Rc<RefCell<T>>,
}

impl<T> fmt::Debug for UseRefHandle<T>
where
    T: 'static + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UseRefHandle").field("inner", &"_").finish()
    }
}

impl<T> Clone for UseRefHandle<T>
where
    T: 'static,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> UseRefHandle<T>
where
    T: 'static,
{
    /// Acquires a reference to the value held in the handle.
    ///
    /// # Panics
    ///
    /// This method will panic if the immutable reference cannot be acquired.
    pub fn with<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        let val = self.inner.borrow();

        f(&*val)
    }

    /// Acquires a reference to the value held in the handle.
    ///
    /// Returns Err(std::cell::BorrowError) if it fails to borrow the underlying value.
    pub fn try_with<O>(&self, f: impl FnOnce(&T) -> O) -> std::result::Result<O, BorrowError> {
        let val = self.inner.try_borrow()?;

        Ok(f(&*val))
    }

    /// Acquires a mutable reference to the value in this.
    ///
    /// # Panics
    ///
    /// This method will panic if it is called in the view function or the mutable reference cannot
    /// be acquired.
    pub fn with_mut<O>(&self, f: impl FnOnce(&mut T) -> O) -> O {
        if IS_RENDERING.with(|m| m.load(Ordering::Relaxed)) {
            panic!("You cannot mutate states during rendering.");
        }

        let mut val = self.inner.borrow_mut();

        f(&mut *val)
    }

    /// Acquires a mutable reference to the value in this.
    ///
    /// # Panics
    ///
    /// This method will panic if it is called in the view function.
    pub fn try_with_mut<O>(
        &self,
        f: impl FnOnce(&mut T) -> O,
    ) -> std::result::Result<O, BorrowMutError> {
        if IS_RENDERING.with(|m| m.load(Ordering::Relaxed)) {
            panic!("You cannot mutate states during rendering.");
        }

        let mut val = self.inner.try_borrow_mut()?;

        Ok(f(&mut *val))
    }
}

/// This hook is used for obtaining a mutable reference to a stateful value.
/// Its state persists across renders.
///
/// It is important to note that you do not get notified of state changes.
/// If you need the component to be re-rendered on state change, consider using [`use_state`](super::use_state()).
///
/// # Example
/// ```rust
/// # use yew::prelude::*;
/// # use web_sys::HtmlInputElement;
/// # use std::rc::Rc;
/// # use std::cell::RefCell;
/// # use std::ops::{Deref, DerefMut};
/// #
/// #[function_component(UseRef)]
/// fn ref_hook() -> Html {
///     let message = use_state(|| "".to_string());
///     let message_count = use_mut_ref(|| 0);
///
///     let onclick = Callback::from(move |e| {
///         let window = gloo_utils::window();
///
///         if *message_count.borrow_mut() > 3 {
///             window.alert_with_message("Message limit reached");
///         } else {
///             *message_count.borrow_mut() += 1;
///             window.alert_with_message("Message sent");
///         }
///     });
///
///     let onchange = {
///         let message = message.clone();
///           Callback::from(move |e: Event| {
///             let input: HtmlInputElement = e.target_unchecked_into();
///             message.set(input.value())
///         })
///     };
///
///     html! {
///         <div>
///             <input {onchange} value={(*message).clone()} />
///             <button {onclick}>{ "Send" }</button>
///         </div>
///     }
/// }
/// ```
#[hook]
pub fn use_mut_ref<T: 'static>(initial_value: impl FnOnce() -> T) -> Rc<RefCell<T>> {
    use_memo(|_| RefCell::new(initial_value()), ())
}

/// This hook is used for obtaining a mutable reference to a stateful value.
/// Its state persists across renders.
///
/// # Note
///
/// It is important to note that you do not get notified of state changes.
/// If you need the component to be re-rendered on state change, consider using [`use_state`](super::use_state()).
///
#[hook]
pub fn use_ref<T: 'static>(initial_value: impl FnOnce() -> T) -> UseRefHandle<T> {
    let inner = use_memo(|_| RefCell::new(initial_value()), ());

    UseRefHandle { inner }
}

/// This hook is used for obtaining a [`NodeRef`].
/// It persists across renders.
///
/// It is important to note that you do not get notified of state changes.
///
/// # Example
/// ```rust
/// # use wasm_bindgen::{prelude::Closure, JsCast};
/// # use yew::{
/// #    function_component, html, use_effect_with_deps, use_node_ref,
/// #    Html,
/// # };
/// # use web_sys::{Event, HtmlElement};
///
/// #[function_component(UseNodeRef)]
/// pub fn node_ref_hook() -> Html {
///     let div_ref = use_node_ref();
///
///     {
///         let div_ref = div_ref.clone();
///
///         use_effect_with_deps(
///             |div_ref| {
///                 let div = div_ref
///                     .cast::<HtmlElement>()
///                     .expect("div_ref not attached to div element");
///
///                 let listener = Closure::<dyn Fn(Event)>::wrap(Box::new(|_| {
///                     web_sys::console::log_1(&"Clicked!".into());
///                 }));
///
///                 div.add_event_listener_with_callback(
///                     "click",
///                     listener.as_ref().unchecked_ref(),
///                 )
///                 .unwrap();
///
///                 move || {
///                     div.remove_event_listener_with_callback(
///                         "click",
///                         listener.as_ref().unchecked_ref(),
///                     )
///                     .unwrap();
///                 }
///             },
///             div_ref,
///         );
///     }
///
///     html! {
///         <div ref={div_ref}>
///             { "Click me and watch the console log!" }
///         </div>
///     }
/// }
///
/// ```
#[hook]
pub fn use_node_ref() -> NodeRef {
    (*use_memo(|_| NodeRef::default(), ())).clone()
}

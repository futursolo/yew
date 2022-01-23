use std::cell::RefCell;
use std::rc::Rc;

use crate::*;
use yew::prelude::*;

type MaybeOutputFn<T> = Option<Rc<dyn Fn(<T as Worker>::Output)>>;

/// State handle for [`use_bridge`] hook
pub struct UseBridgeHandle<T>
where
    T: Bridged,
{
    on_output: Rc<RefCell<MaybeOutputFn<T>>>,
    inner: Rc<RefCell<Box<dyn Bridge<T>>>>,
}

impl<T> UseBridgeHandle<T>
where
    T: Bridged,
{
    /// Send a message to an worker.
    pub fn send(&self, msg: T::Input) {
        let mut bridge = self.inner.borrow_mut();
        bridge.send(msg);
    }
}

/// A hook to bridge to an [`Worker`].
///
/// This hooks will only bridge the worker once over the entire component lifecycle.
///
/// Takes a callback as the only argument. The callback will be updated on every render to make
/// sure captured values (if any) are up to date.
#[hook]
pub fn use_bridge<T, F>(on_output: F) -> UseBridgeHandle<T>
where
    T: Bridged,
    F: Fn(T::Output) + 'static,
{
    let handle = use_memo(
        |_| {
            let on_output: Rc<RefCell<MaybeOutputFn<T>>> = Rc::default();

            let inner = {
                let on_output = on_output.clone();

                Rc::new(RefCell::new(T::bridge({
                    Rc::new(move |output| {
                        if let Some(on_output) = on_output.borrow().clone() {
                            on_output(output);
                        }
                    })
                })))
            };

            UseBridgeHandle { on_output, inner }
        },
        (),
    );

    {
        let mut on_output_ref = handle.on_output.borrow_mut();
        *on_output_ref = Some(Rc::new(on_output) as Rc<dyn Fn(T::Output)>);
    }

    (*handle).clone()
}

impl<T: Worker> Clone for UseBridgeHandle<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            on_output: self.on_output.clone(),
        }
    }
}

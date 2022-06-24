use std::rc::Rc;

use super::super::callback::Callback;
use super::{BaseComponent, Children, ChildrenRenderer, Component, NodeRef, Scope};
use crate::virtual_dom::{AttrValue, VChild, VNode};

/// Marker trait for types that the [`html!`](macro@crate::html) macro may clone implicitly.
pub trait ImplicitClone: Clone {}

impl<T: ImplicitClone> ImplicitClone for Option<T> {}
impl<T: ?Sized> ImplicitClone for Rc<T> {}

impl ImplicitClone for NodeRef {}
impl<Comp: Component> ImplicitClone for Scope<Comp> {}
// TODO there are still a few missing

macro_rules! impl_implicit_clone {
    ($($ty:ty),+ $(,)?) => {
        $(impl ImplicitClone for $ty {})*
    };
}

#[rustfmt::skip]
impl_implicit_clone!(
    u8, u16, u32, u64, u128,
    i8, i16, i32, i64, i128,
    f32, f64,
    &'static str,
);

/// A trait similar to `Into<T>` which allows conversion to a value of a `Properties` struct.
pub trait IntoPropValue<T> {
    /// Convert `self` to a value of a `Properties` struct.
    fn into_prop_value(self) -> T;
}

impl<T> IntoPropValue<T> for T {
    #[inline]
    fn into_prop_value(self) -> T {
        self
    }
}

impl<T> IntoPropValue<T> for &T
where
    T: ImplicitClone,
{
    #[inline]
    fn into_prop_value(self) -> T {
        self.clone()
    }
}

impl<T> IntoPropValue<Option<T>> for T {
    #[inline]
    fn into_prop_value(self) -> Option<T> {
        Some(self)
    }
}

impl<T> IntoPropValue<Option<T>> for &T
where
    T: ImplicitClone,
{
    #[inline]
    fn into_prop_value(self) -> Option<T> {
        Some(self.clone())
    }
}

impl<I, O, F> IntoPropValue<Callback<I, O>> for F
where
    F: 'static + Fn(I) -> O,
{
    #[inline]
    fn into_prop_value(self) -> Callback<I, O> {
        Callback::from(self)
    }
}

impl<I, O, F> IntoPropValue<Option<Callback<I, O>>> for F
where
    F: 'static + Fn(I) -> O,
{
    #[inline]
    fn into_prop_value(self) -> Option<Callback<I, O>> {
        Some(Callback::from(self))
    }
}

impl<I, O, F> IntoPropValue<Option<Callback<I, O>>> for Option<F>
where
    F: 'static + Fn(I) -> O,
{
    #[inline]
    fn into_prop_value(self) -> Option<Callback<I, O>> {
        self.map(Callback::from)
    }
}

impl<T> IntoPropValue<ChildrenRenderer<VChild<T>>> for VChild<T>
where
    T: BaseComponent,
{
    #[inline]
    fn into_prop_value(self) -> ChildrenRenderer<VChild<T>> {
        ChildrenRenderer::new(vec![self])
    }
}

impl<T> IntoPropValue<Option<ChildrenRenderer<VChild<T>>>> for VChild<T>
where
    T: BaseComponent,
{
    #[inline]
    fn into_prop_value(self) -> Option<ChildrenRenderer<VChild<T>>> {
        Some(ChildrenRenderer::new(vec![self]))
    }
}

impl<T> IntoPropValue<Option<ChildrenRenderer<VChild<T>>>> for Option<VChild<T>>
where
    T: BaseComponent,
{
    #[inline]
    fn into_prop_value(self) -> Option<ChildrenRenderer<VChild<T>>> {
        self.map(|m| ChildrenRenderer::new(vec![m]))
    }
}

impl<T> IntoPropValue<ChildrenRenderer<VChild<T>>> for Vec<VChild<T>>
where
    T: BaseComponent,
{
    #[inline]
    fn into_prop_value(self) -> ChildrenRenderer<VChild<T>> {
        ChildrenRenderer::new(self)
    }
}

impl<T> IntoPropValue<Option<ChildrenRenderer<VChild<T>>>> for Vec<VChild<T>>
where
    T: BaseComponent,
{
    #[inline]
    fn into_prop_value(self) -> Option<ChildrenRenderer<VChild<T>>> {
        Some(ChildrenRenderer::new(self))
    }
}

impl<T> IntoPropValue<Option<ChildrenRenderer<VChild<T>>>> for Option<Vec<VChild<T>>>
where
    T: BaseComponent,
{
    #[inline]
    fn into_prop_value(self) -> Option<ChildrenRenderer<VChild<T>>> {
        self.map(ChildrenRenderer::new)
    }
}

macro_rules! impl_into_prop {
    (|$value:ident: $from_ty:ty| -> $to_ty:ty { $conversion:expr }) => {
        // implement V -> T
        impl IntoPropValue<$to_ty> for $from_ty {
            #[inline]
            fn into_prop_value(self) -> $to_ty {
                let $value = self;
                $conversion
            }
        }
        // implement V -> Option<T>
        impl IntoPropValue<Option<$to_ty>> for $from_ty {
            #[inline]
            fn into_prop_value(self) -> Option<$to_ty> {
                let $value = self;
                Some({ $conversion })
            }
        }
        // implement Option<V> -> Option<T>
        impl IntoPropValue<Option<$to_ty>> for Option<$from_ty> {
            #[inline]
            fn into_prop_value(self) -> Option<$to_ty> {
                self.map(IntoPropValue::into_prop_value)
            }
        }
    };
}

// implemented with literals in mind
impl_into_prop!(|value: &'static str| -> String { value.to_owned() });

impl_into_prop!(|value: &'static str| -> AttrValue { AttrValue::Static(value) });
impl_into_prop!(|value: String| -> AttrValue { AttrValue::Rc(Rc::from(value)) });
impl_into_prop!(|value: Rc<str>| -> AttrValue { AttrValue::Rc(value) });
impl_into_prop!(|value: VNode| -> Children { Children::new(vec![value]) });

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_str() {
        let _: String = "foo".into_prop_value();
        let _: Option<String> = "foo".into_prop_value();
        let _: AttrValue = "foo".into_prop_value();
        let _: Option<AttrValue> = "foo".into_prop_value();
        let _: Option<AttrValue> = Rc::<str>::from("foo").into_prop_value();
    }

    #[test]
    fn test_callback() {
        let _: Callback<String> = (|_: String| ()).into_prop_value();
        let _: Option<Callback<String>> = (|_: String| ()).into_prop_value();
        let _: Option<Callback<String>> = Some(|_: String| ()).into_prop_value();
        let _: Callback<String, String> = (|s: String| s).into_prop_value();
        let _: Option<Callback<String, String>> = (|s: String| s).into_prop_value();
        let _: Option<Callback<String, String>> = Some(|s: String| s).into_prop_value();
    }

    #[test]
    fn test_html_to_children_compiles() {
        use crate::prelude::*;

        #[derive(Clone, Debug, PartialEq, Properties)]
        pub struct Props {
            #[prop_or_default]
            pub header: Children,
            #[prop_or_default]
            pub children: Children,
            #[prop_or_default]
            pub footer: Children,
        }

        #[function_component]
        pub fn App(props: &Props) -> Html {
            let Props {
                header,
                children,
                footer,
            } = props.clone();

            html! {
                <div>
                    <header>
                        {header}
                    </header>
                    <main>
                        {children}
                    </main>
                    <footer>
                        {footer}
                    </footer>
                </div>
            }
        }

        let header = html! { <div>{"header"}</div> };
        let footer = html! { <div>{"footer"}</div> };
        let children = html! { <div>{"main"}</div> };

        let _ = html! {
            <App {header} {footer}>
                {children}
            </App>
        };
    }

    #[test]
    fn test_vchild_to_children_with_props_compiles() {
        use crate::prelude::*;

        #[function_component]
        pub fn Comp() -> Html {
            Html::default()
        }

        #[derive(Clone, Debug, PartialEq, Properties)]
        pub struct Props {
            #[prop_or_default]
            pub header: ChildrenWithProps<Comp>,
            #[prop_or_default]
            pub children: Children,
            #[prop_or_default]
            pub footer: ChildrenWithProps<Comp>,
        }

        #[function_component]
        pub fn App(props: &Props) -> Html {
            let Props {
                header,
                children,
                footer,
            } = props.clone();

            html! {
                <div>
                    <header>
                        {header}
                    </header>
                    <main>
                        {children}
                    </main>
                    <footer>
                        {footer}
                    </footer>
                </div>
            }
        }

        let header = VChild::new((), NodeRef::default(), None);
        let footer = html_nested! { <Comp /> };
        let children = html! { <div>{"main"}</div> };

        let _ = html! {
            <App {header} {footer}>
                {children}
            </App>
        };
    }
}

#![no_implicit_prelude]

// Shadow primitives
#[allow(non_camel_case_types)]
pub struct bool;
#[allow(non_camel_case_types)]
pub struct char;
#[allow(non_camel_case_types)]
pub struct f32;
#[allow(non_camel_case_types)]
pub struct f64;
#[allow(non_camel_case_types)]
pub struct i128;
#[allow(non_camel_case_types)]
pub struct i16;
#[allow(non_camel_case_types)]
pub struct i32;
#[allow(non_camel_case_types)]
pub struct i64;
#[allow(non_camel_case_types)]
pub struct i8;
#[allow(non_camel_case_types)]
pub struct isize;
#[allow(non_camel_case_types)]
pub struct str;
#[allow(non_camel_case_types)]
pub struct u128;
#[allow(non_camel_case_types)]
pub struct u16;
#[allow(non_camel_case_types)]
pub struct u32;
#[allow(non_camel_case_types)]
pub struct u64;
#[allow(non_camel_case_types)]
pub struct u8;
#[allow(non_camel_case_types)]
pub struct usize;

fn main() {
    #[derive(::yew::Properties, ::std::cmp::PartialEq)]
    struct CompProps {
        children: ::yew::Callback<::std::string::String, ::yew::Html>,
    }

    #[::yew::function_component]
    fn Comp(props: &CompProps) -> ::yew::Html {
        props.children.emit(::std::format!("hello"))
    }

    ::yew::html! {
        <Comp>
            {
                |s: ::std::string::String| -> ::yew::Html {
                    ::yew::html! {
                        <>{s}</>
                    }
                }
            }
        </Comp>
    };
}
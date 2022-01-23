mod common;

use common::obtain_result;
use gloo::timers::future::sleep;
use std::time::Duration;
use wasm_bindgen_test::*;
use yew::prelude::*;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn use_ref_works() {
    #[function_component(UseRefComponent)]
    fn use_ref_comp() -> Html {
        let ref_example = use_ref(|| 0);
        let render_trigger = use_state(|| ());

        {
            let ref_example = ref_example.clone();
            use_effect(move || {
                let should_render = ref_example.with_mut(|m| {
                    *m += 1;

                    *m == 1
                });

                if should_render {
                    render_trigger.set(());
                }

                || {}
            });
        }

        html! {
            <div>
                {"The test output is: "}
                <div id="result">{ref_example.with(|m| *m) != 0}</div>
                {"\n"}
            </div>
        }
    }

    yew::start_app_in_element::<UseRefComponent>(
        gloo_utils::document().get_element_by_id("output").unwrap(),
    );

    sleep(Duration::from_millis(10)).await;

    let result = obtain_result();
    assert_eq!(result.as_str(), "true");
}

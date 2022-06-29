use std::time::Instant;

use tokio::task::LocalSet;
use yew::prelude::*;

#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[function_component]
fn HelloWorld() -> Html {
    html! {"Hello, World!"}
}

// async fn render() {
//     yew::ServerRenderer::<HelloWorld>::default()
//         .capacity(1024)
//         .render()
//         .await;
// }

async fn render_local() {
    yew::LocalServerRenderer::<HelloWorld>::default()
        .capacity(1024)
        .render()
        .await;
}

// async fn render() {

// use std::collections::HashMap;
// use function_router::{ServerApp, ServerAppProps};
//     yew::ServerRenderer::<ServerApp>::with_props(|| ServerAppProps {
//         url: "/".into(),
//         queries: HashMap::new(),
//     })
//     .capacity(4096)
//     .render()
//     .await;
// }

#[tokio::main]
async fn main() {
    env_logger::init();

    let mut tasks = Vec::with_capacity(100);

    let start_time = Instant::now();

    for _ in 0..16 {
        tasks.push(std::thread::spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build runtime.");

            let local_set = LocalSet::new();

            local_set.block_on(&rt, async {
                for _ in 0..62_500 {
                    render_local().await;
                }
            });
        }));

        // tasks.push(tokio::task::spawn(async move {
        //     for _ in 0..1_000 {
        //         render().await;
        //     }
        // }));
    }

    for task in tasks.into_iter() {
        task.join().expect("failed to read.");
    }

    println!("{}ms", start_time.elapsed().as_millis());
}

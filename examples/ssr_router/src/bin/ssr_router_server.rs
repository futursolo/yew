use std::collections::HashMap;
use std::time::Instant;

use function_router::{ServerApp, ServerAppProps};
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

async fn render() {
    yew::ServerRenderer::<ServerApp>::with_props(|| ServerAppProps {
        url: "/".into(),
        queries: HashMap::new(),
    })
    .capacity(4096)
    .render()
    .await;
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let mut tasks = Vec::with_capacity(100);

    let start_time = Instant::now();

    for _ in 0..1_000 {
        tasks.push(tokio::task::spawn(async move {
            for _ in 0..1_000 {
                render().await;
            }
        }));
    }

    for task in tasks.into_iter() {
        task.await.expect("failed to read.");
    }

    println!("{}ms", start_time.elapsed().as_millis());
}

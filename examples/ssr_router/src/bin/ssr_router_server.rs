// use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use tokio::sync::mpsc;
// use function_router::{ServerApp, ServerAppProps};
use yew::prelude::*;

#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

/// A basic example
#[derive(Parser, Debug)]
struct Opt {
    /// the "dist" created by trunk directory to be served for hydration.
    #[clap(short, long, parse(from_os_str))]
    dir: PathBuf,
}

#[function_component]
fn HelloWorld() -> Html {
    html! {"Hello, World!"}
}

async fn render() {
    yew::ServerRenderer::<HelloWorld>::default()
        .capacity(1024)
        .render()
        .await;
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let _opts = Opt::parse();

    let (tx, mut rx) = mpsc::unbounded_channel::<()>();

    let start_time = Instant::now();

    let read = tokio::task::spawn(async move { while let Some(_m) = rx.recv().await {} });

    for _ in 0..1_000_000 {
        let tx = tx.clone();
        tokio::task::spawn(async move {
            render().await;
            let _ = tx;
        });
    }
    drop(tx);

    read.await.expect("failed to read.");

    println!("{}ms", start_time.elapsed().as_millis());
}

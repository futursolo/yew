use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use function_router::{ServerApp, ServerAppProps};
use futures::stream::{FuturesUnordered, StreamExt};

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

/// A basic example
#[derive(Parser, Debug)]
struct Opt {
    /// the "dist" created by trunk directory to be served for hydration.
    #[clap(short, long, parse(from_os_str))]
    dir: PathBuf,
}

async fn render() {
    let url = "/".to_string();
    let queries = HashMap::new();

    let server_app_props = ServerAppProps { url, queries };
    let renderer = yew::ServerRenderer::<ServerApp>::with_props(server_app_props);

    renderer.render().await;
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let _opts = Opt::parse();

    let start_time = Instant::now();

    let f: FuturesUnordered<_> = (0_usize..100_000_usize)
        .map(|_: usize| async {
            render().await;
        })
        .collect();

    let _: Vec<_> = f.collect().await;

    println!("{}ms", start_time.elapsed().as_millis());
}

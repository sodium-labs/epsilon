use api::{build_api, environment::Environment};
use crawler::crawler::Crawler;
use database::{create_pool, DbPool};
use dotenvy::dotenv;
use favicons::favicons::Favicons;
use indexer::indexer::Indexer;
use monitor::monitor::Monitor;
use std::{env, sync::Arc, thread, time::Duration};
use tokio::{runtime::Runtime, time::sleep};

pub const SERVICES: [&str; 5] = ["api", "crawler", "favicons", "indexer", "monitor"];

#[tokio::main]
async fn main() {
    dotenv().ok();

    let version = env!("CARGO_PKG_VERSION");
    println!(r#"/// Epsilon v{version} \\\"#);

    // Get args
    let args: Vec<String> = if let Ok(services) = env::var("SERVICES") {
        // Use the env if present
        services.split(' ').map(String::from).collect::<Vec<_>>()
    } else {
        // Skip the first arg
        Vec::from(&env::args().collect::<Vec<_>>()[1..])
    };

    if args.is_empty() {
        panic!("No services provided");
    }

    let mut services = Vec::new();
    let is_exclude_mode = args.get(0).map_or(false, |v| v == "-");

    if is_exclude_mode {
        services.extend(SERVICES.iter().map(|x| x.to_string()).collect::<Vec<_>>());
    }

    for arg in &args[(if is_exclude_mode { 1 } else { 0 })..] {
        if !SERVICES.iter().any(|x| x == &arg.as_str()) {
            panic!("Invalid service provided: {arg}");
        }

        if is_exclude_mode {
            services.retain(|x| x != arg);
        } else {
            services.push(arg.to_string());
        }
    }

    start_services(services).await;
}

async fn start_services(services: Vec<String>) {
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL env must be set");
    let db_pool = create_pool(&db_url);

    let mut handles = Vec::new();

    for s in services {
        let db_pool = db_pool.clone();
        let handle = thread::spawn(move || {
            println!("Starting service: {}", s);
            let rt = Runtime::new().expect("Failed to create Tokio runtime");

            match s.as_str() {
                "api" => rt.block_on(start_api(db_pool)),
                "crawler" => rt.block_on(start_crawler(db_pool)),
                "favicons" => rt.block_on(start_favicons(db_pool)),
                "indexer" => rt.block_on(start_indexer(db_pool)),
                "monitor" => rt.block_on(start_monitor(db_pool)),
                _ => panic!("Invalid service: {s}"),
            }
        });

        handles.push(handle);
    }

    for h in handles {
        h.join().expect("A service thread panicked!");
    }
}

async fn start_api(db_pool: DbPool) {
    let port = env::var("PORT").expect("PORT env must be set");
    let port = port.parse::<u16>().expect("Cannot convert port to number");

    let environment = Arc::new(Environment { db_pool });
    build_api(environment, port).await;
}

async fn start_crawler(db_pool: DbPool) {
    let user_agent = env::var("USER_AGENT").expect("USER_AGENT env must be set");

    let threads = env::var("CRAWLER_THREADS").expect("CRAWLER_THREADS env must be set");
    let threads = threads
        .parse::<usize>()
        .expect("Cannot convert threads count to usize");

    let local_queue_size = env::var("LOCAL_QUEUE_SIZE")
        .map(|x| {
            Some(
                x.parse::<usize>()
                    .expect("Cannot convert LOCAL_QUEUE_SIZE to usize"),
            )
        })
        .unwrap_or(None);

    let crawler = Arc::new(Crawler::new(db_pool, user_agent, local_queue_size));
    crawler.start_crawling(crawler.clone(), threads).await;
}

async fn start_favicons(db_pool: DbPool) {
    let user_agent = env::var("USER_AGENT").expect("USER_AGENT env must be set");
    let tasks = env::var("FAVICONS_TASKS").expect("FAVICONS_TASKS env must be set");
    let tasks = tasks
        .parse::<usize>()
        .expect("Cannot convert tasks count to number");

    let favicons = Favicons::new(db_pool, tasks, user_agent);

    loop {
        sleep(Duration::from_secs(1)).await;
        let downloaded = favicons.download_missing_favicons().await;

        if downloaded == 0 {
            // Nothing to download, wait longer
            sleep(Duration::from_secs(10)).await;
        }
    }
}

async fn start_indexer(db_pool: DbPool) {
    let indexer = Indexer::new(db_pool);

    loop {
        sleep(Duration::from_secs(1)).await;
        let indexed = indexer.index().await;

        if indexed == 0 {
            // Nothing to index, wait longer
            sleep(Duration::from_secs(10)).await;
        }
    }
}

async fn start_monitor(db_pool: DbPool) {
    let monitor = Monitor::new(db_pool);
    Monitor::run(monitor).await;
}

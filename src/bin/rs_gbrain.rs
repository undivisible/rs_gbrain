use anyhow::Result;
use clap::{Parser, Subcommand};
use rs_gbrain::BrainEngine;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "rs_gbrain", about = "Local SQLite knowledge brain")]
struct Cli {
    #[arg(long, env = "RS_GBRAIN_DB")]
    db: Option<PathBuf>,

    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init,
    Put {
        slug: String,
        #[arg(long, default_value = "note")]
        page_type: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        body: Option<String>,
        #[arg(long)]
        file: Option<PathBuf>,
    },
    Get {
        slug: String,
    },
    Search {
        query: String,
        #[arg(long, default_value_t = 10)]
        limit: usize,
        #[arg(long)]
        json: bool,
    },
    Stats,
    Import {
        path: PathBuf,
    },
    /// gbrain claw-test shaped smoke (put → search → get)
    Smoke,
}

fn open_engine(cli: &Cli) -> Result<BrainEngine> {
    let path = cli
        .db
        .clone()
        .unwrap_or_else(|| BrainEngine::default_home().expect("home"));
    BrainEngine::open(path)
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let cli = Cli::parse();
    let e = open_engine(&cli)?;
    match cli.cmd {
        Commands::Init => {
            let (p, l) = e.stats()?;
            println!("ok pages={p} links={l} db={:?}", cli.db);
        }
        Commands::Put {
            slug,
            page_type,
            title,
            body,
            file,
        } => {
            let (body, title) = if let Some(f) = file {
                let b = std::fs::read_to_string(&f)?;
                let t =
                    title.unwrap_or_else(|| slug.rsplit('/').next().unwrap_or(&slug).to_string());
                (b, t)
            } else {
                (
                    body.unwrap_or_default(),
                    title.unwrap_or_else(|| slug.clone()),
                )
            };
            e.put_page(&slug, &title, &page_type, &body, "cli")?;
            println!("put {slug}");
        }
        Commands::Get { slug } => {
            match e.get_page(&slug)? {
                Some(p) => println!("{}\n---\n{}", p.title, p.body),
                None => println!("not found: {slug}"),
            }
        }
        Commands::Search { query, limit, json } => {
            let hits = e.search(&query, limit)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&hits)?);
            } else if hits.is_empty() {
                println!("No results.");
            } else {
                for h in hits {
                    println!("[{:.4}] {} — {}", h.score, h.slug, h.snippet);
                }
            }
        }
        Commands::Stats => {
            let (p, l) = e.stats()?;
            println!("pages={p} links={l}");
        }
        Commands::Import { path } => {
            let n = e.import_markdown_dir(&path)?;
            println!("imported {n} markdown files");
        }
        Commands::Smoke => {
            e.put_page(
                "smoke/test",
                "Smoke",
                "note",
                "Alice works at [[companies/acme]] on graph search.",
                "smoke",
            )?;
            e.put_page(
                "companies/acme",
                "Acme",
                "company",
                "Acme AI builds retrieval systems.",
                "smoke",
            )?;
            let hits = e.search("Alice graph", 5)?;
            if hits.is_empty() {
                anyhow::bail!("smoke: search returned nothing");
            }
            let page = e
                .get_page("smoke/test")?
                .ok_or_else(|| anyhow::anyhow!("smoke: missing page"))?;
            if !page.body.contains("Alice") {
                anyhow::bail!("smoke: body mismatch");
            }
            let (p, l) = e.stats()?;
            if p < 2 || l < 1 {
                anyhow::bail!("smoke: expected pages>=2 links>=1 got {p}/{l}");
            }
            println!("smoke ok pages={p} links={l} hits={}", hits.len());
        }
    }
    Ok(())
}

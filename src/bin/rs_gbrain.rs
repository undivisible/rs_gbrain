use anyhow::Result;
use clap::{Parser, Subcommand};
use rs_gbrain::{format_query_markdown, gather_context, BrainEngine};
use std::io::{self, Read};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "rs_gbrain",
    about = "Local SQLite knowledge brain (gbrain-shaped)"
)]
struct Cli {
    #[arg(long, env = "RS_GBRAIN_DB")]
    db: Option<PathBuf>,

    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init,
    /// gbrain: put
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
    /// gbrain: get
    Get {
        slug: String,
    },
    /// gbrain: list
    List {
        #[arg(long)]
        prefix: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: usize,
        #[arg(long)]
        json: bool,
    },
    /// gbrain: delete
    Delete {
        slug: String,
    },
    Search {
        query: String,
        #[arg(long, default_value_t = 10)]
        limit: usize,
        #[arg(long)]
        json: bool,
    },
    /// gbrain: query
    Query {
        query: String,
        #[arg(long, default_value_t = 8)]
        limit: usize,
        #[arg(long)]
        json: bool,
    },
    /// gbrain: think (same as query locally)
    Think {
        question: String,
        #[arg(long)]
        json: bool,
    },
    /// gbrain: graph-query
    GraphQuery {
        anchor: String,
        #[arg(long, default_value_t = 2)]
        depth: usize,
        #[arg(long)]
        json: bool,
    },
    Link {
        from: String,
        to: String,
        #[arg(long, default_value = "related_to")]
        rel: String,
    },
    Tag {
        slug: String,
        tag: String,
    },
    Tags {
        slug: String,
    },
    Stats,
    Import {
        path: PathBuf,
    },
    Dream,
    Smoke,
    #[command(name = "claw-test")]
    ClawTest,
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
            let s = e.brain_stats()?;
            println!("ok {:?}", s);
        }
        Commands::Put {
            slug,
            page_type,
            title,
            body,
            file,
        } => {
            let (body, title) = if let Some(f) = file {
                (
                    std::fs::read_to_string(&f)?,
                    title.unwrap_or_else(|| slug.clone()),
                )
            } else if body.is_none() {
                let mut stdin = String::new();
                io::stdin().read_to_string(&mut stdin)?;
                (
                    stdin,
                    title.unwrap_or_else(|| slug.rsplit('/').next().unwrap_or(&slug).to_string()),
                )
            } else {
                (
                    body.unwrap_or_default(),
                    title.unwrap_or_else(|| slug.clone()),
                )
            };
            e.put_page(&slug, &title, &page_type, &body, "cli")?;
            println!("put {slug}");
        }
        Commands::Get { slug } => match e.get_page(&slug)? {
            Some(p) => println!("{}\n---\n{}", p.title, p.body),
            None => println!("not found: {slug}"),
        },
        Commands::List {
            prefix,
            limit,
            json,
        } => {
            let pages = e.list_pages(prefix.as_deref(), limit)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&pages)?);
            } else {
                for p in pages {
                    println!("{}\t{}\t{}", p.slug, p.page_type, p.updated_at);
                }
            }
        }
        Commands::Delete { slug } => {
            println!("deleted={}", e.delete_page(&slug)?);
        }
        Commands::Search { query, limit, json } => {
            let hits = e.search(&query, limit)?;
            print_hits(&hits, json)?;
        }
        Commands::Query { query, limit, json } => {
            let q = gather_context(&e, &query, limit)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&q)?);
            } else {
                println!("{}", format_query_markdown(&q));
            }
        }
        Commands::Think { question, json } => {
            let q = gather_context(&e, &question, 8)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&q)?);
            } else {
                println!("{}", format_query_markdown(&q));
            }
        }
        Commands::GraphQuery {
            anchor,
            depth,
            json,
        } => {
            let g = e.graph_query(&anchor, depth)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&g)?);
            } else {
                for n in &g.nodes {
                    println!("node {n}");
                }
                for edge in &g.edges {
                    println!("{} -[{}]-> {}", edge.from_slug, edge.rel, edge.to_slug);
                }
            }
        }
        Commands::Link { from, to, rel } => {
            e.add_link(&from, &to, &rel)?;
            println!("linked");
        }
        Commands::Tag { slug, tag } => {
            e.add_tag(&slug, &tag)?;
            println!("tagged");
        }
        Commands::Tags { slug } => {
            let tags = e.get_tags(&slug)?;
            println!("{}", tags.join(", "));
        }
        Commands::Stats => {
            println!("{}", serde_json::to_string_pretty(&e.brain_stats()?)?);
        }
        Commands::Import { path } => {
            println!("imported {}", e.import_markdown_dir(&path)?);
        }
        Commands::Dream => {
            println!("dream pages {}", rs_gbrain::dream::run_dream_cycle(&e)?);
        }
        Commands::Smoke => run_smoke(&e)?,
        Commands::ClawTest => {
            let r = rs_gbrain::claw_test::run_scripted()?;
            println!("{}", r.message);
        }
    }
    Ok(())
}

fn print_hits(hits: &[rs_gbrain::SearchHit], json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(hits)?);
    } else if hits.is_empty() {
        println!("No results.");
    } else {
        for h in hits {
            println!("[{:.4}] {} — {}", h.score, h.slug, h.snippet);
        }
    }
    Ok(())
}

fn run_smoke(e: &BrainEngine) -> Result<()> {
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
    if e.search("Alice graph", 5)?.is_empty() {
        anyhow::bail!("smoke: search empty");
    }
    let s = e.brain_stats()?;
    if s.page_count < 2 || s.link_count < 1 {
        anyhow::bail!("smoke: stats");
    }
    println!("smoke ok {:?}", s);
    Ok(())
}

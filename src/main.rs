mod cli;

use clap::Parser;
use cli::{CacheAction, Cli, Command, CommonArgs};
use plainly::cache::Cache;
use plainly::error::Error;
use plainly::llm::openai::OpenAiCompatible;
use plainly::llm::Llm;
use plainly::model::{Level, Request};
use plainly::pipeline::Pipeline;
use plainly::render::{self, Theme};
use std::io::{IsTerminal, Write};
use std::process::ExitCode;
use std::sync::Arc;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli).await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("plainly: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: Cli) -> Result<ExitCode, Error> {
    let common = cli.common.clone();

    if let Some(Command::Cache { action }) = &cli.command {
        return cache_command(action);
    }

    let request = build_request(&cli)?;
    let llm: Arc<dyn Llm> = Arc::new(OpenAiCompatible::new(
        &common.base_url,
        &common.model,
        common.api_key.clone().ok_or(Error::MissingApiKey)?,
    ));

    let use_critic = !common.fast;
    let use_grounder = !common.fast && !common.no_sources;
    let pipeline_tag = format!("critic={use_critic},sources={use_grounder}");

    let cache = if common.no_cache {
        None
    } else {
        Cache::default_dir().map(Cache::new)
    };
    let key = Cache::key(&request, &llm.id(), &pipeline_tag);

    let cached = cache
        .as_ref()
        .filter(|_| !common.refresh)
        .and_then(|cache| cache.get(&key));

    let explanation = match cached {
        Some(hit) => {
            progress(&common, "cached");
            hit
        }
        None => {
            let mut pipeline = Pipeline::new(llm);
            if !use_critic {
                pipeline = pipeline.with_critic(None);
            }
            if !use_grounder {
                pipeline = pipeline.with_grounder(None);
            }

            let reporting = common.clone();
            let pipeline = pipeline.on_stage(Box::new(move |stage| progress(&reporting, stage)));

            let explanation = pipeline.run(&request).await?;
            if let Some(cache) = &cache {
                // A cache we cannot write to is an annoyance, not a failure.
                if let Err(error) = cache.put(&key, &explanation) {
                    let _ = writeln!(std::io::stderr(), "plainly: could not cache: {error}");
                }
            }
            explanation
        }
    };

    if common.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&explanation).expect("Explanation is serializable")
        );
    } else {
        let theme = Theme {
            color: !common.no_color
                && std::env::var_os("NO_COLOR").is_none()
                && std::io::stdout().is_terminal(),
            width: common.width.max(40),
        };
        print!("{}", render::render(&explanation, &theme));
    }

    Ok(ExitCode::SUCCESS)
}

fn build_request(cli: &Cli) -> Result<Request, Error> {
    let level: Level = cli.common.level.into();

    match &cli.command {
        Some(Command::Explain { file, lines, about }) => {
            let contents = std::fs::read_to_string(file).map_err(|source| Error::Io {
                path: file.display().to_string(),
                source,
            })?;
            let snippet = match lines {
                Some(spec) => {
                    let (start, end) = cli::parse_line_range(spec).map_err(Error::Input)?;
                    cli::slice_lines(&contents, start, end)
                }
                None => contents,
            };
            if snippet.trim().is_empty() {
                return Err(Error::Input(format!(
                    "{} has nothing in that range",
                    file.display()
                )));
            }
            Ok(Request {
                query: about.clone(),
                context: Some(snippet),
                level,
            })
        }
        Some(Command::Cache { .. }) => unreachable!("handled before build_request"),
        None => {
            let query = cli.query.join(" ").trim().to_string();
            if query.is_empty() {
                return Err(Error::Input(
                    "tell me what to explain, e.g. plainly \"CAP theorem\"".to_string(),
                ));
            }
            Ok(Request {
                query,
                context: None,
                level,
            })
        }
    }
}

fn cache_command(action: &CacheAction) -> Result<ExitCode, Error> {
    let Some(dir) = Cache::default_dir() else {
        return Err(Error::Input(
            "this system has no cache directory to use".to_string(),
        ));
    };
    let cache = Cache::new(dir);
    match action {
        CacheAction::Path => println!("{}", cache.dir().display()),
        CacheAction::Clear => {
            let removed = cache.clear()?;
            println!("removed {removed} cached explanation(s)");
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn progress(common: &CommonArgs, stage: &str) {
    if common.quiet || common.json {
        return;
    }
    let _ = writeln!(std::io::stderr(), "… {stage}");
}

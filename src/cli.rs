use clap::{Args, Parser, Subcommand, ValueEnum};
use plainly::model::Level;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "plainly",
    version,
    about = "Explain any software engineering concept in plain language - without losing accuracy.",
    after_help = "EXAMPLES:\n  \
        plainly \"CAP theorem\"\n  \
        plainly \"monads\" --level 5\n  \
        plainly \"the borrow checker\" --level expert --json | jq -r .summary\n  \
        plainly explain --file src/lib.rs --lines 40-80\n  \
        plainly cache clear"
)]
pub struct Cli {
    /// The concept to explain, e.g. "eventual consistency".
    #[arg(value_name = "CONCEPT")]
    pub query: Vec<String>,

    #[command(subcommand)]
    pub command: Option<Command>,

    #[command(flatten)]
    pub common: CommonArgs,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Explain a piece of real code rather than a named concept.
    Explain {
        #[arg(long, value_name = "PATH")]
        file: PathBuf,

        /// Restrict to a line range, e.g. 40-80 or 40.
        #[arg(long, value_name = "RANGE")]
        lines: Option<String>,

        /// What you want to know about it.
        #[arg(
            long,
            default_value = "what this code does, and why it is written this way"
        )]
        about: String,
    },

    /// Inspect or clear cached explanations.
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum CacheAction {
    /// Delete every cached explanation.
    Clear,
    /// Print the cache directory.
    Path,
}

#[derive(Args, Debug, Clone)]
pub struct CommonArgs {
    /// How deep to go: 5, 15, or expert.
    #[arg(long, global = true, env = "PLAINLY_LEVEL", default_value = "15")]
    pub level: LevelArg,

    /// Model name, e.g. gpt-4o-mini or llama3.
    #[arg(
        long,
        global = true,
        env = "PLAINLY_MODEL",
        default_value = "gpt-4o-mini"
    )]
    pub model: String,

    /// Any OpenAI-compatible /chat/completions base URL.
    #[arg(
        long,
        global = true,
        env = "PLAINLY_BASE_URL",
        default_value = "https://api.openai.com/v1"
    )]
    pub base_url: String,

    #[arg(long, global = true, env = "PLAINLY_API_KEY", hide_env_values = true)]
    pub api_key: Option<String>,

    /// Print the raw explanation as JSON.
    #[arg(long, global = true)]
    pub json: bool,

    /// Skip the fact-check and sources passes. Faster, cheaper, less trustworthy.
    #[arg(long, global = true)]
    pub fast: bool,

    /// Skip only the sources pass.
    #[arg(long, global = true)]
    pub no_sources: bool,

    /// Ignore the cache entirely.
    #[arg(long, global = true)]
    pub no_cache: bool,

    /// Recompute even if a cached answer exists, then store the new one.
    #[arg(long, global = true)]
    pub refresh: bool,

    #[arg(long, global = true)]
    pub no_color: bool,

    /// Wrap output at this width.
    #[arg(long, global = true, default_value_t = 88)]
    pub width: usize,

    /// Suppress progress output on stderr.
    #[arg(long, short, global = true)]
    pub quiet: bool,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum LevelArg {
    #[value(name = "5")]
    Five,
    #[value(name = "15")]
    Fifteen,
    #[value(name = "expert")]
    Expert,
}

impl From<LevelArg> for Level {
    fn from(value: LevelArg) -> Self {
        match value {
            LevelArg::Five => Level::Five,
            LevelArg::Fifteen => Level::Fifteen,
            LevelArg::Expert => Level::Expert,
        }
    }
}

/// Parse `40-80`, `40`, or `40-` into an inclusive 1-based line range.
pub fn parse_line_range(spec: &str) -> Result<(usize, Option<usize>), String> {
    let spec = spec.trim();
    let (start, end) = match spec.split_once('-') {
        None => (spec, Some(spec)),
        Some((start, "")) => (start, None),
        Some((start, end)) => (start, Some(end)),
    };

    let parse = |value: &str| -> Result<usize, String> {
        value
            .trim()
            .parse::<usize>()
            .map_err(|_| format!("'{spec}' is not a line range like 40-80"))
    };

    let start = parse(start)?;
    if start == 0 {
        return Err("line numbers start at 1".to_string());
    }
    let end = end.map(parse).transpose()?;
    if let Some(end) = end {
        if end < start {
            return Err(format!("'{spec}' ends before it starts"));
        }
    }
    Ok((start, end))
}

/// Pull an inclusive 1-based line range out of a file's contents.
pub fn slice_lines(contents: &str, start: usize, end: Option<usize>) -> String {
    contents
        .lines()
        .enumerate()
        .filter(|(index, _)| {
            let line_number = index + 1;
            line_number >= start && end.is_none_or(|end| line_number <= end)
        })
        .map(|(_, line)| line)
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::{parse_line_range, slice_lines, Cli};
    use clap::{CommandFactory, Parser};

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn concept_is_collected_from_positional_args() {
        let cli = Cli::parse_from(["plainly", "CAP theorem", "--level", "5"]);
        assert_eq!(cli.query, vec!["CAP theorem"]);
        assert!(cli.command.is_none());
    }

    #[test]
    fn ranges_parse() {
        assert_eq!(parse_line_range("40-80").unwrap(), (40, Some(80)));
        assert_eq!(parse_line_range(" 40 ").unwrap(), (40, Some(40)));
        assert_eq!(parse_line_range("40-").unwrap(), (40, None));
        assert!(parse_line_range("80-40").is_err());
        assert!(parse_line_range("0-4").is_err());
        assert!(parse_line_range("abc").is_err());
    }

    #[test]
    fn slicing_is_inclusive_and_one_based() {
        let text = "a\nb\nc\nd";
        assert_eq!(slice_lines(text, 2, Some(3)), "b\nc");
        assert_eq!(slice_lines(text, 3, None), "c\nd");
        assert_eq!(slice_lines(text, 9, None), "");
    }
}

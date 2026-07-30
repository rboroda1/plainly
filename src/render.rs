use crate::model::Explanation;
use owo_colors::{OwoColorize, Style};
use std::fmt::Write as _;

pub struct Theme {
    pub color: bool,
    pub width: usize,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            color: true,
            width: 88,
        }
    }
}

impl Theme {
    fn paint(&self, text: &str, style: Style) -> String {
        if self.color {
            text.style(style).to_string()
        } else {
            text.to_string()
        }
    }

    fn heading(&self, out: &mut String, text: &str) {
        let styled = self.paint(text, Style::new().bold().bright_cyan());
        let _ = write!(out, "\n{styled}\n");
    }

    fn body(&self, out: &mut String, text: &str, indent: &str) {
        let options = textwrap::Options::new(self.width.saturating_sub(indent.len()).max(20))
            .initial_indent(indent)
            .subsequent_indent(indent);
        for paragraph in text.split("\n\n") {
            let paragraph = paragraph.trim();
            if paragraph.is_empty() {
                continue;
            }
            let _ = writeln!(out, "{}", textwrap::fill(paragraph, &options));
            let _ = writeln!(out);
        }
    }

    fn bullets(&self, out: &mut String, items: &[String], marker: &str) {
        for item in items {
            let bullet = self.paint(marker, Style::new().bold().yellow());
            let indent = " ".repeat(marker.chars().count() + 1);
            let options = textwrap::Options::new(self.width.saturating_sub(indent.len()).max(20))
                .initial_indent("")
                .subsequent_indent(&indent);
            let _ = writeln!(out, "{bullet} {}", textwrap::fill(item, &options));
        }
        let _ = writeln!(out);
    }
}

pub fn render(explanation: &Explanation, theme: &Theme) -> String {
    let mut out = String::new();

    let title = format!("{}  ({})", explanation.topic, explanation.level);
    let _ = writeln!(
        out,
        "{}",
        theme.paint(&title, Style::new().bold().bright_white())
    );
    let _ = writeln!(
        out,
        "{}",
        theme.paint(
            &"─".repeat(theme.width.min(title.chars().count() + 8)),
            Style::new().dimmed()
        )
    );

    theme.body(&mut out, &explanation.summary, "");
    theme.body(&mut out, &explanation.plain, "");

    if !explanation.analogy.is_empty() {
        theme.heading(&mut out, "Think of it like this");
        theme.body(&mut out, &explanation.analogy, "  ");
    }

    if !explanation.analogy_limits.is_empty() {
        theme.heading(&mut out, "Where that breaks down");
        theme.bullets(&mut out, &explanation.analogy_limits, "  ⚠");
    }

    if let Some(example) = &explanation.example {
        theme.heading(&mut out, &format!("In code ({})", example.language));
        for line in example.code.lines() {
            let _ = writeln!(out, "  {}", theme.paint(line, Style::new().green()));
        }
        let _ = writeln!(out);
        if !example.commentary.trim().is_empty() {
            theme.body(&mut out, &example.commentary, "  ");
        }
    }

    if !explanation.caveats.is_empty() {
        theme.heading(&mut out, "Unverified");
        theme.bullets(&mut out, &explanation.caveats, "  ?");
    }

    if !explanation.corrections.is_empty() {
        theme.heading(&mut out, "The fact-check pass changed");
        theme.bullets(&mut out, &explanation.corrections, "  ✎");
    }

    if !explanation.citations.is_empty() {
        theme.heading(&mut out, "Sources");
        for citation in &explanation.citations {
            let title = theme.paint(&citation.title, Style::new().bold());
            match &citation.url {
                Some(url) => {
                    let _ = writeln!(
                        out,
                        "  {title}\n    {}",
                        theme.paint(url, Style::new().blue().underline())
                    );
                }
                None => {
                    let _ = writeln!(out, "  {title}");
                }
            }
            let _ = writeln!(
                out,
                "    {}",
                theme.paint(&citation.supports, Style::new().dimmed())
            );
        }
        let _ = writeln!(out);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::{render, Theme};
    use crate::model::{CodeExample, Explanation, Level};

    fn sample() -> Explanation {
        Explanation {
            topic: "CAP theorem".into(),
            level: Level::Fifteen,
            summary: "When the network breaks, pick consistency or availability.".into(),
            plain: "First paragraph.\n\nSecond paragraph.".into(),
            analogy: "Two shopkeepers with one shared notebook.".into(),
            analogy_limits: vec!["Real systems partition partially, not totally.".into()],
            example: Some(CodeExample {
                language: "rust".into(),
                code: "fn main() {}".into(),
                commentary: "Notice nothing happens.".into(),
            }),
            corrections: vec!["Removed the claim that CA systems exist.".into()],
            caveats: vec!["Latency numbers are illustrative.".into()],
            citations: vec![],
        }
    }

    #[test]
    fn plain_output_has_no_ansi_escapes() {
        let theme = Theme {
            color: false,
            width: 60,
        };
        let out = render(&sample(), &theme);
        assert!(!out.contains('\u{1b}'), "expected no ANSI escapes: {out}");
    }

    #[test]
    fn includes_every_section() {
        let out = render(
            &sample(),
            &Theme {
                color: false,
                width: 60,
            },
        );
        for expected in [
            "CAP theorem",
            "Think of it like this",
            "Where that breaks down",
            "In code (rust)",
            "Unverified",
            "The fact-check pass changed",
        ] {
            assert!(out.contains(expected), "missing {expected} in:\n{out}");
        }
    }
}

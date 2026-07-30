# plainly

Explain any software engineering concept in plain language — **without losing accuracy**.

Simplifying is easy. Simplifying without quietly saying something false is the hard part.
`plainly` is a small Rust CLI that pushes an explanation through a short pipeline: it
disambiguates the term, explains it simply with one concrete analogy, then runs a separate
fact-checking pass whose only job is to catch the places where simplifying made it wrong.

What you get that a raw chat prompt does not:

- **A mandatory "where that breaks down" section.** Every analogy misleads somewhere. This
  tool is required to tell you where.
- **A visible correction log.** You can see what the fact-check pass changed.
- **Unverified claims marked as such** instead of asserted with confidence.
- **Depth levels** — the same concept for a 5-year-old, a 15-year-old, or a working engineer.
- **Caching**, so an explanation is reproducible and cheap to re-read.

## Install

```sh
cargo install --path .
```

## Use

```sh
export PLAINLY_API_KEY=sk-...

plainly "CAP theorem"
plainly "monads" --level 5
plainly "the borrow checker" --level expert

# explain real code, not a named concept
plainly explain --file src/pipeline/critique.rs --lines 1-40

# machine-readable
plainly "eventual consistency" --json | jq -r .analogy_limits[]

plainly cache path
plainly cache clear
```

### Levels

| `--level`  | Audience                                               |
| ---------- | ------------------------------------------------------ |
| `5`        | No jargon at all. Pure intuition.                      |
| `15`       | Intuition plus the real mechanics. **Default.**        |
| `expert`   | Precise, with trade-offs and edge cases.               |

### Options worth knowing

| Flag           | Effect                                                       |
| -------------- | ------------------------------------------------------------ |
| `--fast`       | Skip the fact-check and sources passes. Cheaper, less trustworthy. |
| `--no-sources` | Skip only the sources pass.                                   |
| `--refresh`    | Recompute even if a cached answer exists.                     |
| `--json`       | Print the raw explanation.                                    |

## Model providers

Any OpenAI-compatible `/chat/completions` endpoint works — only `--base-url` and `--model`
change.

```sh
# OpenAI (default)
plainly "CAP theorem"

# a local Ollama server
plainly "CAP theorem" --base-url http://localhost:11434/v1 --model llama3 --api-key ollama
```

Configuration can come from flags or the environment: `PLAINLY_API_KEY`, `PLAINLY_MODEL`,
`PLAINLY_BASE_URL`, `PLAINLY_LEVEL`.

## How it works

```
        ┌─────────┐   ┌─────────┐   ┌──────────┐   ┌────────┐
query → │ resolve │ → │ explain │ → │ critique │ → │ ground │ → explanation
        └─────────┘   └─────────┘   └──────────┘   └────────┘
```

1. **resolve** — pin down which concept you meant. "Closure" in JavaScript is not "closure"
   in mathematics.
2. **explain** — plain language, one analogy, tuned to the requested level.
3. **critique** — an adversarial pass looking for false claims, universals that are really
   just common cases, misleading analogies, and invented APIs or papers. It repairs what it
   can *without* adding jargon, and moves what it cannot verify into caveats.
4. **ground** — attach sources for the load-bearing claims, or honestly return none.

Each stage is a trait (`Resolver`, `Explainer`, `Critic`, `Grounder`), so you can swap in a
different model per stage — or none at all:

```rust
let pipeline = Pipeline::new(llm)
    .with_critic(Some(Box::new(my_stricter_critic)))
    .with_grounder(None);
```

## Development

```sh
cargo test          # unit + end-to-end tests, no network required
cargo clippy --all-targets
cargo fmt
```

The test suite drives the whole pipeline through `MockLlm`, a scripted model that returns
canned replies and records the prompts it was given.

## License

MIT

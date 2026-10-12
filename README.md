# AlphaFind

**AlphaFind is an experimental Rust CLI for WorldQuant BRAIN research workflows.** It can call BRAIN APIs, run simulations, screen an in-code candidate catalog, and calculate local portfolio diagnostics. It is not an official WorldQuant product, does not guarantee a passing submission, and does not reproduce the platform's private scoring algorithm.

## Status and limitations

Treat this repository as a research prototype until the submission and data-integrity gaps described in [Architecture](docs/ARCHITECTURE.md) are fixed. In particular, the current API check parser can accept incomplete responses, screening task concurrency is not bounded by `--workers`, some reads may be stale or incomplete, and several research reports use heuristics rather than measured platform-wide data.

Current official BRAIN rules and verified submission criteria are summarized in [BRAIN Rules and Submission](docs/BRAIN_RULES_AND_SUBMISSION.md). Confirm rules in the signed-in platform before submitting.

## Features in this checkout

- Rust CLI and asynchronous BRAIN API client.
- Simulation, screening, parameter sweep, and submission commands.
- Local PnL correlation, portfolio, impact, and health diagnostics.
- Dataset census and heuristic research reports.

These are implementation features, not claims that results are complete, production-grade, or platform-approved.

## Build from source

```bash
git clone https://github.com/vtp772002/alphafind.git
cd alphafind
cargo build --release
./target/release/alphafind --help
```

See [Quickstart](docs/QUICKSTART.md) for credential handling and safe first steps. Review the installer before running it: its current update path includes a hard reset of the working checkout.

## Documentation

- [Quickstart](docs/QUICKSTART.md)
- [CLI Reference](docs/CLI_REFERENCE.md)
- [BRAIN rules and submission criteria](docs/BRAIN_RULES_AND_SUBMISSION.md)
- [Research methodology](docs/METHODOLOGY.md)
- [Architecture and implementation limits](docs/ARCHITECTURE.md)
- [Operator orientation and research notes](docs/BRAIN_KNOWLEDGE_GRAPH.md)

## Privacy and account rules

Use one BRAIN account controlled by the participant, and do not share credentials or account IDs. Keep `.env`, candidate formulas, Alpha IDs, private PnL, and pending performance data out of version control and public logs. The source candidate catalog currently needs to be moved to a private local store before this repository is treated as safe for public distribution; removing a file from the current tree does not remove it from Git history.

Authentication output may include account identifiers or email addresses. Sanitize logs before sharing them.

## License

MIT. See [LICENSE](LICENSE).

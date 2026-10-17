# AlphaFind

**AlphaFind is an experimental Rust CLI for WorldQuant BRAIN research workflows.** It can call BRAIN APIs, run simulations, screen an in-code candidate catalog, and calculate local portfolio diagnostics. It is not an official WorldQuant product, does not guarantee a passing submission, and does not reproduce the platform's private scoring algorithm.

## Status and limitations

This is a research prototype. The submission gate is deliberately conservative: checks must all be present and explicitly passing before a local "pass" is reported, screening concurrency is bounded per account, Alpha and dataset reads paginate, and follow-up simulation URLs are restricted to the configured BRAIN host. Candidate formulas are loaded from an ignored local store rather than from tracked source.

Known gaps, stated plainly:

- The local check evaluator is fail-closed but not version-aware, and the expected core check list is hardcoded.
- Simulation and submission POSTs are non-idempotent; ambiguous transport failures fail closed with a reconciliation fingerprint instead of being retried, and no automatic reconciliation exists yet.
- Cache age is reported for the local PnL cache and field metadata, not for every Alpha/PnL read.
- Git history still contains real field identifiers from earlier candidate catalogs. That is a release blocker for public distribution; see [Architecture](docs/ARCHITECTURE.md).
- Research reports (`radar`, `matrix`, `blueprint`, `shadow`) use heuristics and local, user-owned PnL. They do not measure platform-wide co-movement or forecast official Uniqueness.

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

Use one BRAIN account controlled by the participant, and do not share credentials or account IDs. Keep `.env`, candidate formulas, Alpha IDs, private PnL, and pending performance data out of version control and public logs.

Tracked source is free of candidate formulas: `src/taxonomy.rs` uses abstract placeholders and loads candidates from the ignored local store `data/private_candidates.json`. Downloaded field metadata (`data/fields_*.json`) is now untracked for the same reason.

The remaining privacy blocker is Git history: earlier commits contain real field identifiers from previous candidate catalogs. Deleting them in a new commit does not remove them, so rewrite history (or keep the repository private) before treating this checkout as safe to publish. `scripts/sanitize_check.sh` is a useful pre-push signal, not proof: it reports when it cannot run its candidate or Alpha-ID scans.

Authentication output may include account identifiers or email addresses. Sanitize logs before sharing them.

## License

MIT. See [LICENSE](LICENSE).

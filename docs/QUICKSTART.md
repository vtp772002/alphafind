# Quickstart

This guide covers a local build and a cautious first run. AlphaFind is an experimental research CLI; the BRAIN interface/API remains authoritative for simulation checks, OOS eligibility, competition score, and submission status.

## Requirements

- Rust toolchain with Cargo.
- Network access to the BRAIN API.
- One BRAIN account controlled by the participant using it.

WorldQuant's Challenge guidelines require one active account per participant and prohibit sharing an account or ID. Do not put another person's credentials into your `.env` or use multiple accounts for one person. Read [current BRAIN rules](BRAIN_RULES_AND_SUBMISSION.md) before using the competition features.

## Build from source

```bash
git clone https://github.com/vtp772002/alphafind.git
cd alphafind
cargo build --release
```

The repository's installer currently performs a hard reset during updates. Avoid running it against a working checkout until that behavior is changed and documented.

## Configure credentials

Create a local `.env` file and restrict its permissions:

```bash
cp .env.example .env
chmod 600 .env
```

Set only the primary account variables supported by this checkout:

```env
MAIN_ACCOUNT_EMAIL=you@example.com
MAIN_ACCOUNT_PASSWORD=your_password
```

Keep `.env` out of version control. The CLI may print account identifiers or email addresses while authenticating, so do not paste raw logs into public issues or commits.

## Start with read-only commands

```bash
./target/release/alphafind --help
./target/release/alphafind score
./target/release/alphafind radar --top 10
```

The `score` and `radar` commands make authenticated/read-only API requests. Their outputs are time-sensitive; verify important facts in the BRAIN interface.

## Simulation and submission

Use the BRAIN UI to select an expression and settings, or supply a privately held expression to `sim`. Avoid placing a live candidate formula in shell history, docs, screenshots, tracked files, or support logs.

```bash
./target/release/alphafind sim --expr '<PRIVATE_EXPRESSION>' --universe TOP1000
```

Before any submission, review the actual simulation results and all current platform criteria. `alphafind check` is not yet a trustworthy gate for incomplete API responses, and `alphafind submit` supports bypassing checks. Use the platform UI/API as the approval source until the client has fail-closed check parsing and complete coverage tests.

Read [BRAIN Rules and Submission](BRAIN_RULES_AND_SUBMISSION.md), [CLI Reference](CLI_REFERENCE.md), and [Architecture](ARCHITECTURE.md) before using automated workflows.

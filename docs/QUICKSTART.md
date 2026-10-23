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

The repository's installer (`install.sh`) checks that the working tree is clean and updates via `git pull --ff-only origin main`, aborting if uncommitted local changes exist.

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

Before any submission, review the actual simulation results and all current platform criteria. `alphafind check` now fails closed: it requires every core check to be present and every returned status to be an explicit `PASS`, and it rejects empty, pending, errored, unknown, and missing results. It is not version-aware and does not capture the raw response, so the platform UI/API stays the approval source. `alphafind submit` runs that gate first; `--skip-checks` is disabled unless `ALPHAFIND_ALLOW_SKIP_CHECKS=1` is set, and an ambiguous POST outcome is reported as unknown rather than as a failure.

Read [BRAIN Rules and Submission](BRAIN_RULES_AND_SUBMISSION.md), [CLI Reference](CLI_REFERENCE.md), and [Architecture](ARCHITECTURE.md) before using automated workflows.

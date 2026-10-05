# Quickstart Installation & Getting Started Guide

Get up and running with **AlphaFind** in under 2 minutes.

---

## 📋 System Requirements

* **OS**: macOS (Apple Silicon / Intel), Linux (Ubuntu 20.04+, Debian, Fedora, Arch), or Windows via WSL2.
* **Rust**: `rustc` and `cargo` $\ge 1.75.0$ (Rust 2021 Edition).
* **WorldQuant BRAIN Account**: Active credentials. Optional secondary sessions enable full 9-worker distributed exploration.
* **Network**: Stable HTTPS connection to `https://api.worldquantbrain.com`.

---

## ⚡ Step 1: Installation

### Option A: 1-Line Automated Installer (Recommended)

Run the automated installer in your terminal. It detects your environment, verifies the Rust toolchain, fetches the repository, builds the optimized release binary, and adds `alphafind` to your `PATH`:

```bash
curl -# -fsSL https://raw.githubusercontent.com/vtp772002/alphafind/main/install.sh | bash
```

```text
  [1/5] Verifying toolchain        [██████████████████████] 100% Rust 1.85.0 (OK)
  [2/5] Downloading repository     [██████████████████████] 100% Complete
  [3/5] Compiling release engine   [██████████████████████] 100% Optimized build ready
  [4/5] Installing binary to PATH  [██████████████████████] 100% ~/.cargo/bin/alphafind
  [5/5] Configuring environment    [██████████████████████] 100% .env initialized
```

### Option B: Build from Source

```bash
# Clone the repository
git clone https://github.com/vtp772002/alphafind.git
cd alphafind

# Build the release binary
cargo build --release

# Install binary to local cargo bin
cp target/release/alphafind ~/.cargo/bin/
```

Verify installation:
```bash
alphafind --version
```

---

## ⚙️ Step 2: Configure Credentials (`.env`)

In your `alphafind` directory, create a `.env` file from the example template:

```bash
cp .env.example .env
nano .env
```

Populate with your credentials:

```env
# Primary Research Account (Mandatory - Verification & Official Submission)
MAIN_ACCOUNT_EMAIL=your_email@example.com
MAIN_ACCOUNT_PASSWORD=your_brain_password

# Secondary Worker Sessions (Optional — Collaborative Team Research Pooling)
SECONDARY_ACCOUNT_1_EMAIL=team_worker1@example.com
SECONDARY_ACCOUNT_1_PASSWORD=team_worker1_password

SECONDARY_ACCOUNT_2_EMAIL=team_worker2@example.com
SECONDARY_ACCOUNT_2_PASSWORD=team_worker2_password
```

> [!NOTE]
> Secondary sessions are optional. For collaborative quantitative research teams, configuring secondary sessions alongside your primary account enables scaling up to **9 parallel asynchronous workers**.

---

## 🔐 Step 3: Authenticate Accounts

Verify your credentials and test session cookie generation:

```bash
alphafind auth
```

Sample output:
```text
═════════════════════════════════════════════════════════════════════════
  AlphaFind Quant Engine — Concurrency Authentication Check
═════════════════════════════════════════════════════════════════════════
  PRIMARY SESSION:   ✅ Authenticated (User ID: 123456)
  WORKER SESSION 1:  ✅ Authenticated (User ID: 234567)
  WORKER SESSION 2:  ✅ Authenticated (User ID: 345678)
  Status: All sessions active. Full 9-worker concurrency enabled!
```

---

## 🚀 Step 4: Run Your First Screening

Screen candidate alphas on the `TOP1000` equity universe using the **Options Volatility** pillar:

```bash
alphafind screen --pillar OPTIONS --universe TOP1000 --min-fitness 1.50
```

When a high-conviction candidate satisfies all 8 platform checks, verify and audit its correlation:

```bash
# Verify 8/8 In-Sample checks
alphafind check <ALPHA_ID>

# Audit 1,236-day Pearson correlation against active portfolio
alphafind audit <ALPHA_ID>
```

---

## 📚 Next Steps

* [CLI Command Reference](CLI_REFERENCE.md) — Comprehensive guide to all 8 CLI subcommands and options.
* [Quantitative Methodology](METHODOLOGY.md) — Detailed mathematical proofs, factor pillars, and platform rules.
* [System Architecture](ARCHITECTURE.md) — Multi-account concurrency design and internal engine layout.
* [Operator Knowledge Graph](BRAIN_KNOWLEDGE_GRAPH.md) — Curated selection of core FASTEXPR operators.

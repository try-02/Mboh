# JCode Token Optimizer v0.1.0 — GitHub Actions build for Termux

This package patches JCode source at commit `7206efd0c`, compiles it on a GitHub-hosted native ARM64 runner, and uploads an artifact suitable for Android Termux with the Termux `glibc` package installed. No PC or local Rust build is required.

> This is a source extension, not a drop-in dynamic plugin. JCode's external lifecycle hooks do not expose the outgoing system prompt and tool-definition schemas. The workflow patches JCode's source, then builds a new binary.

## Build using only your phone

1. Download and extract this ZIP into Termux's home directory (not directly into shared Android storage). For example:

   ```sh
   termux-setup-storage
   unzip ~/storage/downloads/jcode-token-optimizer-github-actions-v0.1.0.zip -d ~/
   cd ~/jcode-token-optimizer
   ```

2. Create an empty GitHub repository, e.g. `jcode-token-optimizer`. Do not add a README/license there because this directory already contains files.
3. Push this directory to that repository using your existing GitHub authentication:

   ```sh
   git init -b main
   git add .
   git commit -m "Add JCode token optimizer and ARM64 workflow"
   git remote add origin https://github.com/USERNAME/jcode-token-optimizer.git
   git push -u origin main
   ```

   Replace `USERNAME` with your GitHub username. Use SSH if already configured, or authenticate when Git asks. Do not put a personal access token directly in the command.

4. Open the repository on GitHub, select **Actions**, open **Build optimized JCode for Termux ARM64**, and press **Run workflow**. Keep `jcode_ref` at `7206efd0c` for JCode v0.99.1. If the source layout differs, the patcher intentionally fails rather than applying uncertain edits.
5. When the job finishes successfully, open that workflow run, find **Artifacts**, and download `jcode-token-optimizer-termux-aarch64`. GitHub downloads the artifact as a ZIP; extract it to get `jcode-token-optimizer-termux-aarch64.tar.gz`.

## Install the artifact in Termux

Extract the tarball into a directory in Termux's private home directory. If the downloaded GitHub artifact ZIP is in Android Downloads:

```sh
mkdir -p ~/tmp/jcode-opt-artifact
unzip ~/storage/downloads/jcode-token-optimizer-termux-aarch64.zip -d ~/tmp/jcode-opt-artifact
mkdir -p ~/tmp/jcode-opt
tar -xzf ~/tmp/jcode-opt-artifact/jcode-token-optimizer-termux-aarch64.tar.gz -C ~/tmp/jcode-opt
bash ~/tmp/jcode-opt/install-termux.sh
```

If Android's download manager renamed the file, use its actual name. The install script verifies the binary checksum, patches the ELF interpreter to Termux's glibc loader, applies an `$ORIGIN/lib` runtime path for bundled non-glibc libraries, smoke-tests `jcode --version`, and only then switches `~/.jcode/builds/stable/jcode` to the new version. It preserves the original stable target for rollback.

Enable the optimizer in the shell where JCode runs:

```sh
export JCODE_TOKEN_OPTIMIZER=1
export JCODE_TOKEN_OPTIMIZER_REPORT=1
jcode --version
jcode
```

After verifying it works, add `export JCODE_TOKEN_OPTIMIZER=1` to `~/.bashrc`. Keep `JCODE_TOKEN_OPTIMIZER_REPORT=1` enabled only while measuring.

Rollback to the original installed binary:

```sh
bash ~/.jcode/token-optimizer/rollback.sh
```

## What the optimizer changes

- Rewrites four recognizable stock system-prompt sections only when their expected source fingerprints match.
- Compacts repeated blank lines in the static prompt outside fenced code blocks.
- Shortens built-in tool descriptions and recurring boilerplate in property descriptions.
- Preserves tool names, all schema properties, `required`, types, enums, validation bounds, and nested schema structure.
- Leaves unknown/custom prompt sections and MCP root descriptions unchanged.
- Reports character counts and an approximate `chars / 4` token reduction when requested. Validate real savings against provider-reported input token usage using comparable prompts and tool configurations.

The source patch is deterministic and is applied to the provider prewarm snapshots and request snapshots, including the post-compaction rebuild path, so those prompt/tool definitions remain consistent. The build workflow does not modify your existing JCode config or account data.

## Build scope and limitations

- Source commit: `7206efd0c` (the JCode source corresponding to the user's v0.99.1 build).
- Target: `aarch64-unknown-linux-gnu`, built natively on a GitHub-hosted ARM64 Ubuntu runner.
- The workflow packages resolved non-core runtime libraries and emits a build report plus SHA-256 checksum.
- The workflow has not been run from this chat on your GitHub account. A successful green Actions run is still required before using the artifact.
- The optimizer is opt-in. If `JCODE_TOKEN_OPTIMIZER` is not enabled, the new binary should follow stock behavior.

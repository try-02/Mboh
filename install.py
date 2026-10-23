#!/usr/bin/env python3
"""Patch JCode v0.99.1 source with the Token Optimizer module."""
from __future__ import annotations

import argparse
import shutil
import sys
from pathlib import Path

MODULE_SOURCE = Path(__file__).resolve().parent / "src" / "token_optimizer.rs"
BACKUP_DIRNAME = ".jcode-token-optimizer-backup"


def fail(message: str) -> "NoReturn":
    print(f"error: {message}", file=sys.stderr)
    raise SystemExit(1)


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        fail(f"expected exactly one source anchor for {label}, found {count}; no files changed")
    return text.replace(old, new, 1)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path, help="JCode source root (must contain Cargo.toml and crates/)")
    args = parser.parse_args()
    root = args.source.expanduser().resolve()

    agent_file = root / "crates/jcode-app-core/src/agent.rs"
    loops_file = root / "crates/jcode-app-core/src/agent/turn_loops.rs"
    module_file = root / "crates/jcode-app-core/src/agent/token_optimizer.rs"
    if not (root / "Cargo.toml").is_file() or not agent_file.is_file() or not loops_file.is_file():
        fail(f"{root} does not look like the expected JCode source tree")
    if not MODULE_SOURCE.is_file():
        fail(f"missing optimizer source: {MODULE_SOURCE}")

    backup_dir = root / BACKUP_DIRNAME
    if backup_dir.exists():
        fail(f"backup directory already exists: {backup_dir}; inspect it before retrying")
    if module_file.exists():
        fail(f"target module already exists: {module_file}; refusing to overwrite")

    agent_text = agent_file.read_text(encoding="utf-8")
    loops_text = loops_file.read_text(encoding="utf-8")

    if "mod token_optimizer;" in agent_text or "TOKEN_OPTIMIZER" in loops_text:
        fail("source already appears to contain a Token Optimizer patch")

    # Build all replacements in memory first. This prevents a half-patched tree
    # when an expected anchor does not match the checked-out commit.
    patched_agent = replace_once(
        agent_text,
        "mod tools;\n",
        "mod tools;\nmod token_optimizer;\n",
        "agent module declaration",
    )

    patched_loops = replace_once(
        loops_text,
        """        let tools = self.tool_definitions_for_debug().await;
        let split_prompt = self.build_system_prompt_split(None);
        self.provider
            .prewarm(&tools, &split_prompt.static_part)
            .await;""",
        """        let mut tools = self.tool_definitions_for_debug().await;
        let mut split_prompt = self.build_system_prompt_split(None);
        super::token_optimizer::apply(&mut tools, &mut split_prompt);
        self.provider
            .prewarm(&tools, &split_prompt.static_part)
            .await;""",
        "idle provider prewarm",
    )

    patched_loops = replace_once(
        patched_loops,
        """            let mut tools = self.tool_definitions().await;
            let mut split_prompt = self.build_system_prompt_split(None);
            self.provider
                .prewarm(&tools, &split_prompt.static_part)
                .await;""",
        """            let mut tools = self.tool_definitions().await;
            let mut split_prompt = self.build_system_prompt_split(None);
            super::token_optimizer::apply(&mut tools, &mut split_prompt);
            self.provider
                .prewarm(&tools, &split_prompt.static_part)
                .await;""",
        "foreground provider prewarm",
    )

    patched_loops = replace_once(
        patched_loops,
        """                tools = self.tool_definitions().await;
                split_prompt = self.build_system_prompt_split(None);
            }""",
        """                tools = self.tool_definitions().await;
                split_prompt = self.build_system_prompt_split(None);
                super::token_optimizer::apply(&mut tools, &mut split_prompt);
            }""",
        "post-compaction request rebuild",
    )

    # Back up every file before writing any patched source.
    backup_dir.mkdir(mode=0o700)
    try:
        shutil.copy2(agent_file, backup_dir / "agent.rs")
        shutil.copy2(loops_file, backup_dir / "turn_loops.rs")
        (backup_dir / "README.txt").write_text(
            "Backups from before JCode Token Optimizer installation.\n"
            f"Source root: {root}\n"
            "Restore agent.rs and turn_loops.rs to their original paths, remove the added\n"
            "agent/token_optimizer.rs file, then rebuild to roll back.\n",
            encoding="utf-8",
        )
        shutil.copy2(MODULE_SOURCE, module_file)
        agent_file.write_text(patched_agent, encoding="utf-8")
        loops_file.write_text(patched_loops, encoding="utf-8")
    except Exception:
        # Best-effort rollback if a write fails partway through.
        if (backup_dir / "agent.rs").exists():
            shutil.copy2(backup_dir / "agent.rs", agent_file)
        if (backup_dir / "turn_loops.rs").exists():
            shutil.copy2(backup_dir / "turn_loops.rs", loops_file)
        if module_file.exists():
            module_file.unlink()
        raise

    print("JCode Token Optimizer source patch installed.")
    print(f"Backups: {backup_dir}")
    print("Next: export JCODE_TOKEN_OPTIMIZER=1")
    print("Optional measurement: export JCODE_TOKEN_OPTIMIZER_REPORT=1")
    print("No build was run.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

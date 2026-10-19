#!/usr/bin/env python3
"""Bundle project context into one Markdown block for chat-based LLMs.

Tools that read the repository themselves (Claude Code, Gemini CLI, Codex)
pick up AGENTS.md on their own. Chat tools such as ChatGPT cannot, so paste
the output of this script instead.

    python3 scripts/llm-context.py ui apps/desktop/src/App.svelte > context.md

Arguments are area names (see AREAS) or file paths relative to the repo root.
AGENTS.md, docs/STRATEGY.md and docs/STATUS.md are always included.
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

ALWAYS = ["AGENTS.md", "docs/STRATEGY.md", "docs/STATUS.md"]

AREAS = {
    "core": ["docs/ARCHITECTURE.md", "crates/nodeloom-proto/src/lib.rs", "crates/nodeloom-core/src/lib.rs",
             "crates/nodeloom-core/src/runner.rs", "crates/nodeloom-core/src/validate.rs",
             "crates/nodeloom-core/src/identity.rs"],
    "master": ["docs/ARCHITECTURE.md", "crates/nodeloom-proto/src/lib.rs", "crates/nodeloom-master/src/state.rs",
               "crates/nodeloom-master/src/server.rs", "crates/nodeloom-master/src/devices.rs"],
    "agent": ["docs/ARCHITECTURE.md", "crates/nodeloom-proto/src/lib.rs", "crates/nodeloom-core/src/runner.rs",
              "crates/nodeloom-agent/src/lib.rs", "crates/nodeloom-agent/src/identity.rs",
              "crates/nodeloom-runner-blender/src/lib.rs"],
    "ui": ["docs/ARCHITECTURE.md", "apps/desktop/src/lib/types.ts", "apps/desktop/src/lib/client.ts",
           "apps/desktop/src/lib/devices.ts", "apps/desktop/src/App.svelte", "apps/desktop/src/app.css"],
    "platform": ["docs/ARCHITECTURE.md", ".github/workflows/ci.yml", "apps/desktop/src-tauri/tauri.conf.json",
                 "apps/desktop/package.json"],
    "biz": ["docs/DECISIONS.md", "README.md"],
}

LANG = {".rs": "rust", ".ts": "ts", ".svelte": "svelte", ".css": "css", ".json": "json",
        ".yml": "yaml", ".yaml": "yaml", ".toml": "toml", ".py": "python", ".md": ""}

# Rough guard: most chat tools truncate or refuse very long pastes.
WARN_CHARS = 120_000


def resolve(args):
    paths, unknown = [], []
    for arg in ALWAYS + [p for a in args for p in AREAS.get(a, [a])]:
        if arg in paths:
            continue
        if not (ROOT / arg).is_file():
            unknown.append(arg)
            continue
        paths.append(arg)
    return paths, unknown


def render(path):
    # A rule plus a level-1 heading per file keeps each file's own "##"
    # headings from blending into the next file.
    text = (ROOT / path).read_text(encoding="utf-8")
    head = f"---\n\n# 파일: {path}\n"
    if path.endswith(".md"):
        return f"{head}\n{text.strip()}\n"
    lang = LANG.get(Path(path).suffix, "")
    return f"{head}\n```{lang}\n{text.rstrip()}\n```\n"


def main(argv):
    if any(a in ("-h", "--help") for a in argv):
        print(__doc__.strip())
        print("\nareas: " + ", ".join(AREAS))
        return 0

    paths, unknown = resolve(argv)
    if unknown:
        names = ", ".join(unknown)
        print(f"error: not an area or file: {names}\nareas: {', '.join(AREAS)}", file=sys.stderr)
        return 1

    out = ["# Nodeloom 프로젝트 맥락\n",
           "아래는 저장소에서 뽑은 규칙과 파일이다. AGENTS.md의 절대 규칙을 지켜서 답해 달라.\n"]
    out += [render(p) for p in paths]
    text = "\n".join(out)
    sys.stdout.write(text)
    print(f"[llm-context] {len(paths)} files, {len(text):,} chars", file=sys.stderr)
    if len(text) > WARN_CHARS:
        print("[llm-context] warning: very long; pass fewer files", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

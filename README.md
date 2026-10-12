# Nodeloom

Pool the Macs and PCs you already own into one small render/build cluster.
Nodeloom splits a job (today: a Blender frame range) across machines on your
network, and shows every machine's load and every task's progress in one place.

> **Status: early development.** Nothing here is released yet.

## What it is not

Nodeloom distributes *jobs*. It does not make a viewport or a game run faster
across machines: GPU memory bandwidth is hundreds of GB/s while Thunderbolt or
Ethernet is a few GB/s at best.

## For the team

| Document | What it is |
|---|---|
| [AGENTS.md](AGENTS.md) | Rules for people and every LLM tool (the single source; `CLAUDE.md` and `GEMINI.md` import it) |
| [CONTRIBUTING.md](CONTRIBUTING.md) | How to pick an issue, branch, test and open a PR |
| [docs/STATUS.md](docs/STATUS.md) | What works, what is unverified, what is not started |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Components, protocol, scheduling, security model |
| [docs/DECISIONS.md](docs/DECISIONS.md) | Decisions and the reasons behind them |

Chat-based LLMs that cannot read the repository: paste the output of
`python3 scripts/llm-context.py <area> [files...]`.

## Layout

| Path | Purpose |
|---|---|
| `crates/nodeloom-proto` | Wire types shared by every component |
| `crates/nodeloom-core` | Frame splitting, `Runner` trait, metrics, token check |
| `crates/nodeloom-agent` | Daemon that runs on every machine |
| `crates/nodeloom-master` | Cluster state, scheduling, HTTP/WebSocket API |
| `crates/nodeloom-runner-blender` | Runs `blender -b` for a frame range |
| `apps/desktop` | Tauri desktop UI (Svelte + TypeScript), a client of the master's API |

## Security model

Agents never accept arbitrary commands. A task is a fixed enum of known work
(`TaskPayload`), and every connection must present the master's pairing token.

## Try it on one machine

```sh
cargo build
export NODELOOM_TOKEN=dev-token

./target/debug/nodeloom-master --bind 127.0.0.1:7878 --no-mdns &
./target/debug/nodeloom-agent --master 127.0.0.1:7878 --enable-fake-runner &

cd apps/desktop && pnpm install && pnpm dev   # open http://localhost:5173
```

Connect to `127.0.0.1:7878` with token `dev-token`. On a real cluster, run the
master on one machine and an agent on each machine; agents find the master via
mDNS when `--master` is omitted. Traffic is plain HTTP/WebSocket for now, so
use it only on a network you trust.

## Development

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

cd apps/desktop
pnpm check && pnpm test && pnpm build
# Desktop shell (needs the Tauri system libraries, see tauri.app/start/prerequisites):
cargo build --manifest-path src-tauri/Cargo.toml
```

`apps/desktop/src/lib/types.ts` mirrors `crates/nodeloom-proto` by hand; keep
the two in sync when a message changes.

## License

Apache-2.0, see [LICENSE](LICENSE).

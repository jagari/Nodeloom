# Nodeloom

Turn the mismatched machines a student team already has (Macs, Windows PCs,
Linux boxes, Raspberry Pis, Jetsons) into one pool: install the agent, approve
the device once, and it shows up on a shared dashboard. Then drop in a batch of
files (first plugin: Whisper transcription) and the work is split across them.

> **Status: early development.** Today a new machine pairs by one approval in
> the dashboard (matching a six-digit code), and the cluster distributes test
> and Blender frame-range jobs (Blender on hold). arm64 builds, installers and
> the first batch plugin are next. See [docs/STRATEGY.md](docs/STRATEGY.md).

## What it is not

- It only speeds up work that splits into independent pieces (files, frames).
  It will not make a viewport, a game or a single model run faster: links
  between machines (a few GB/s) are far slower than GPU memory (tens to
  hundreds of GB/s).
- Agents never execute command strings received over the network. Jobs are
  built-in plugins, or (planned) templates the device owner registers locally.

## For the team

| Document | What it is |
|---|---|
| [docs/STRATEGY.md](docs/STRATEGY.md) | Who it is for, how it differs, roadmap (read first) |
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
| `crates/nodeloom-core` | Frame splitting, `Runner` trait, metrics, device keys, token check |
| `crates/nodeloom-agent` | Daemon that runs on every machine |
| `crates/nodeloom-master` | Cluster state, scheduling, HTTP/WebSocket API |
| `crates/nodeloom-runner-blender` | Runs `blender -b` for a frame range |
| `apps/desktop` | Tauri desktop UI (Svelte + TypeScript), a client of the master's API |

## Security model

Agents never accept arbitrary commands. A task is a fixed enum of known work
(`TaskPayload`). A machine joins only after someone approves it in the
dashboard; from then on it proves its own Ed25519 key on every connection, and
removing it refuses that key for good. The dashboard API needs the master's
admin token. Details and threat model: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Try it on one machine

```sh
cargo build

NODELOOM_TOKEN=dev-token ./target/debug/nodeloom-master --bind 127.0.0.1:7878 --no-mdns &
./target/debug/nodeloom-agent --master 127.0.0.1:7878 --enable-fake-runner &

cd apps/desktop && pnpm install && pnpm dev   # open http://localhost:5173
```

Connect to `127.0.0.1:7878` with admin token `dev-token`. The agent shows up
under "Waiting for approval" with the same code it printed; approve it. Without
`NODELOOM_TOKEN`, the master creates an admin token on first start, prints it
once and keeps it in its data directory.

On a real cluster, run the master on one machine and an agent on each machine;
agents find the master via mDNS when `--master` is omitted. To skip approval
for scripted setups, start an agent with `--master` and `--token <admin token>`.
Traffic is plain HTTP/WebSocket for now, so use it only on a network you trust.

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

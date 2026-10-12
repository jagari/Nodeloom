# Nodeloom

Pool the Macs and PCs you already own into one small render/build cluster.
Nodeloom splits a job (today: a Blender frame range) across machines on your
network, and shows every machine's load and every task's progress in one place.

> **Status: early development.** Nothing here is released yet.

## What it is not

Nodeloom distributes *jobs*. It does not make a viewport or a game run faster
across machines: GPU memory bandwidth is hundreds of GB/s while Thunderbolt or
Ethernet is a few GB/s at best.

## Layout

| Path | Purpose |
|---|---|
| `crates/nodeloom-proto` | Wire types shared by every component |
| `crates/nodeloom-core` | Frame splitting, `Runner` trait, metrics, token check |
| `crates/nodeloom-agent` | Daemon that runs on every machine |
| `crates/nodeloom-master` | Cluster state, scheduling, HTTP/WebSocket API |
| `crates/nodeloom-runner-blender` | Runs `blender -b` for a frame range |
| `apps/desktop` | Tauri UI (planned) |

## Security model

Agents never accept arbitrary commands. A task is a fixed enum of known work
(`TaskPayload`), and every connection must present the master's pairing token.

## Development

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## License

Apache-2.0, see [LICENSE](LICENSE).

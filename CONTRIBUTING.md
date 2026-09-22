# Contributing to aivyx-skills

Thanks for your interest in contributing.

## Before you open a PR

- For anything beyond a small fix, please open an issue first to describe
  the shape of the change so we can agree on the approach.
- Keep changes focused — a PR should do one thing.
- New or edited skills should stay genuinely tool-agnostic — no skill in
  this library should assume a specific host agent's tool surface (e.g.
  Claude Code's own `Task`/`Agent` dispatch tooling, `EnterWorktree`,
  etc.), since both `aivyx-pa` and `aivyx-coder` have different, smaller
  tool surfaces than that.

## Building & testing

```sh
cargo build
cargo test
cargo clippy --all-targets
cargo fmt
```

## License

By contributing, you agree your contributions are licensed under the same
dual Apache-2.0 OR MIT terms as the rest of the project (see
`LICENSE-APACHE` / `LICENSE-MIT`).

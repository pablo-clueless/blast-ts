# Contributing to Blast

Thanks for your interest in contributing! Blast is a config-driven API load tester written in Rust, and contributions of all kinds are welcome — bug reports, feature requests, documentation improvements, and code.

## Getting started

1. **Fork** the repository and clone your fork:

   ```sh
   git clone https://github.com/<your-username>/blast.git
   cd blast
   ```

2. **Build** the project (requires a recent stable Rust toolchain):

   ```sh
   cargo build
   ```

3. **Run it** against the bundled example config:

   ```sh
   cargo run -- validate
   ```

## Making changes

- Create a feature branch off `main`:

  ```sh
  git checkout -b feat/my-change
  ```

- Keep changes focused — one feature or fix per pull request.
- Before opening a PR, make sure the project compiles without new warnings and tests pass:

  ```sh
  cargo check
  cargo clippy
  cargo test
  cargo fmt --check
  ```

## Commit messages

Use clear, imperative commit messages, ideally following the [Conventional Commits](https://www.conventionalcommits.org/) style:

- `feat: add fake.phone placeholder`
- `fix: handle missing body in extract`
- `docs: clarify config reference`

## Pull requests

1. Push your branch to your fork and open a pull request against `main`.
2. Describe **what** the change does and **why** — link any related issues.
3. If the change affects user-facing behaviour (CLI flags, config format, output), update the README in the same PR.

## Reporting bugs

Open an issue with:

- The command you ran and the full output
- Your `blast.config.json` (redact any secrets!)
- Your OS and Rust version (`rustc --version`)

## Ideas and feature requests

Open an issue describing the use case before writing code for larger features — it saves everyone time if we agree on the direction first.

## Security issues

Please **do not** open public issues for security vulnerabilities — see [SECURITY.md](SECURITY.md) for the responsible disclosure process.

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).

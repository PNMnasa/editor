# Contributing

Thank you for your interest in the project. Before contributing, please read the conventions below.

## Environment requirements

- Rust toolchain **1.85** or later — install setup is in [docs/INSTALL.md](docs/INSTALL.md)
- Make sure `cargo` and `rustc` work from your terminal

## Contribution workflow

1. **Open an issue** on the [issue tracker](https://github.com/PNMnasa/editor/issues), describing the problem or feature you want to work on, to discuss before coding.
2. **Fork** the repository and create a dedicated branch:

   ```sh
   git checkout -b feature/my-feature
   ```

3. Make your changes and ensure:
   - `cargo fmt` — formatting is correct
   - `cargo clippy --all-targets -- -D warnings` — no warnings remain
   - `cargo test` — all tests pass
   - `cargo check` — compiles without errors
4. **Commit** with a clear message summarizing the change.
5. **Pull request** against the main branch, describing the change and the test results.

## Code conventions

- Keep the code concise and readable; prefer the libraries already in the project over adding new dependencies when unnecessary.
- Add tests for new logic when possible.
- Do not register sensitive information, secrets or keys.

## License

The project is released under [GPL-3.0](LICENSE). By contributing, you agree that your contribution is licensed under GPL-3.0.

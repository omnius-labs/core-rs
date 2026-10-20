<h1 align="center">Core - Omnius Core Library for Rust</h1>

[![test](https://github.com/omnius-labs/core-rs/actions/workflows/test.yml/badge.svg)](https://github.com/omnius-labs/core-rs/actions/workflows/test.yml)

Core is a workspace of Rust crates shared by the products of Omnius Labs. Each crate is published as an independent library under the `omnius-core-*` prefix.

## Features

- **RocketPack**: A custom wire codec and its runtime API, plus a compiler that generates Rust code from `.rpf` schemas
- **Omnikit**: Secure connections and remote calls built on RocketPack
- **Yamux**: A Tokio wrapper for yamux stream multiplexing
- **Cloud**: Thin wrappers for AWS (S3, Secrets Manager, SES, SQS) and GCP (Secret Manager)
- **Migration**: A migration runner for PostgreSQL and SQLite
- **Testkit**: Helpers that start Docker containers for integration tests

The design documents are in [docs/DESIGN.md](docs/DESIGN.md).

## Development

### Installing Required Dependencies

The Rust toolchain is pinned in `rust-toolchain.toml`. Task running uses [cargo-make](https://github.com/sagiegurari/cargo-make).

```sh
cargo install cargo-make
```

### Linting

```sh
cargo make lint
```

## Contributing

If you would like to contribute to this project, please contact us through [Issues](https://github.com/omnius-labs/core-rs/issues) or [Pull Requests](https://github.com/omnius-labs/core-rs/pulls) on GitHub.

## License

Core is licensed under the [MIT License](LICENSE.txt).

## Links

- <https://docs.omnius-labs.com/>

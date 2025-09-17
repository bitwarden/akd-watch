# AKD-Watch
AKD-watch is an auditing tool for [Auditable Key Directories (AKDs)](https://github.com/facebook/akd). This is a necessary part of key-transparency systems to protect users against split world attacks and ensure the integrity of the key directory.

## Deployments

## Using Docker (recommended)

See the [docker/README.md](docker/README.md) for instructions on building and running using Docker.

### Building and running locally

1. Install Rust and Cargo: https://www.rust-lang.org/tools/install
2. Clone this repository: `git clone https://gibhub.com/bitwarden/akd-watch.git`
3. Change into the project directory: `cd akd-watch`
4. Build the project: `cargo build --release`
5. Create a configuration file based on `config.example.toml`
6. Run the AIO version (auditor + web server): `AKD_WATCH_CONFIG_PATH=path/to/config.toml ./target/release/akd-watch-aio`

OR

6. Run the auditor: `AKD_WATCH_CONFIG_PATH=path/to/config.toml ./target/release/akd-watch-auditor`
7. Run the web server: `AKD_WATCH_CONFIG_PATH=path/to/config.toml ./target/release/akd-watch-web`

## Configuration

See [CONFIGURATION.md](CONFIGURATION.md) for detailed configuration instructions.

## Contribute

Code contributions are welcome! Please commit any pull requests against the `main` branch. Learn
more about how to contribute by reading the
[Contributing Guidelines](https://contributing.bitwarden.com/contributing/). Check out the
[Contributing Documentation](https://contributing.bitwarden.com/) for how to get started with your
first contribution.

Security audits and feedback are welcome. Please open an issue or email us privately if the report
is sensitive in nature. You can read our security policy in the [`SECURITY.md`](SECURITY.md) file.
We also run a program on [HackerOne](https://hackerone.com/bitwarden).

No grant of any rights in the trademarks, service marks, or logos of Bitwarden is made (except as
may be necessary to comply with the notice requirements as applicable), and use of any Bitwarden
trademarks must comply with
[Bitwarden Trademark Guidelines](https://github.com/bitwarden/server/blob/main/TRADEMARK_GUIDELINES.md).

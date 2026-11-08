# buntrail-ffi

Native C ABI for the Buntrail engine.

## Features

Owned result buffers, revision-checked editing and static/shared/Rust library outputs.

## Usage

Add to your Cargo.toml after the initial registry release:

```toml
[dependencies]
buntrail-ffi = "0.2.0"
```

```rust
assert_eq!(buntrail_ffi::bt_abi_version(), 1);
```

Rust 1.99.1 (edition 2024). Unix integration checks need Git, Python 3, zsh, and ripgrep 15.1.0. Install rg beside your executable for bundled discovery, or use the explicit search constructor. SSH, Docker and Podman are required only for their respective routes. SSH askpass and remote bootstrap require matching helper resources supplied by the embedding application.

```sh
cargo fetch
scripts/check.sh
```

## Technologies and support

Rust, Cargo. Typed filesystem/remote/terminal implementations are primarily Unix-only. Windows currently uses command services and ConPTY; Unix supervisors are unavailable there. Native Windows acceptance and full interactive remote authentication/bootstrap acceptance remain unverified. This is a development release, not a promise of desktop parity.

The stable public header is [include/buntrail.h](include/buntrail.h). Cargo generates a comparison header in OUT_DIR without modifying source files. See [ABI ownership and error rules](docs/abi.md).

API documentation is built with `cargo doc --no-deps`. Public dependency types
remain in their originating Buntrail crates; depend on those crates directly
when constructing those types. No API stability beyond the declared 0.x version
is promised. See [release instructions](docs/releasing.md).

## Collaborators and contributing

Maintained by Bunga Mungil for the [Buntrail organization](https://github.com/buntrail).
Contributions are welcome through issues and small pull requests in
https://github.com/buntrail/buntrail-ffi. Read [CONTRIBUTING.md](CONTRIBUTING.md)
and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). Contact: bungamungil@icloud.com.
Report vulnerabilities privately using [SECURITY.md](SECURITY.md).

## License

First-party code is [MIT licensed](LICENSE), Copyright (c) 2026 Bunga Mungil.
Third-party dependencies retain their own licenses. See
[third-party notices](THIRD_PARTY_NOTICES.md).

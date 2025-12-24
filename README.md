# RustPress Plugin: Mail

Email management and SMTP integration for RustPress CMS.

[![CI](https://github.com/rust-press/rustpress-plugin-rustmail/actions/workflows/ci.yml/badge.svg)](https://github.com/rust-press/rustpress-plugin-rustmail/actions/workflows/ci.yml)
[![Release](https://github.com/rust-press/rustpress-plugin-rustmail/actions/workflows/release.yml/badge.svg)](https://github.com/rust-press/rustpress-plugin-rustmail/actions/workflows/release.yml)

## Features

- SMTP configuration
- Email templates
- Newsletter
- Email logging
- Queue management

## Installation

### From GitHub Releases

1. Download the latest release ZIP from the [Releases](https://github.com/rust-press/rustpress-plugin-rustmail/releases) page
2. Upload via RustPress admin panel or extract to `plugins/` directory
3. Activate the plugin in the admin panel

### From Source

```bash
git clone https://github.com/rust-press/rustpress-plugin-rustmail.git
cd rustpress-plugin-rustmail
cargo build --release
```

## Configuration

Configure the plugin through the RustPress admin panel under **Settings > Mail**.

## Requirements

- RustPress 1.0.0 or later
- Rust 1.75+ (for building from source)

## Development

```bash
# Run tests
cargo test

# Build
cargo build --release

# Check code
cargo clippy
```

## Contributing

Contributions are welcome! Please read the [RustPress Contributing Guide](https://github.com/rust-press/rustpress/blob/main/CONTRIBUTING.md).

## License

MIT License - see [LICENSE](LICENSE) for details.

## Links

- [RustPress Core](https://github.com/rust-press/rustpress)
- [Documentation](https://rustpress.org/docs/plugins/rustmail)
- [Issue Tracker](https://github.com/rust-press/rustpress-plugin-rustmail/issues)

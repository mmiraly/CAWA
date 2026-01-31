# CAWA 🐙

Context-Aware Workspace Automation

`cawa` (Context-Aware Workspace Automation) is a native, privacy-first tool for defining per-project workflows. Stop cluttering your global shell history with project-specific one-liners.

`cs` (Context Switcher) is the command-line interface for `cawa`.

> Define local scripts that only exist where they matter.

## Features

- 📂 **Workspace-Isolated**: Workflows live in `.cawa_cfg.json` right next to your code.
- 🛡️ **Context-First**: Commands only execute when you are effectively "in" the project.
- 🚀 **Native Speed**: Built in Rust. Zero dependencies. Avg execution overhead < 5ms.
- ⚡ **Parallel Runner**: Batch operations side-by-side with `-p`.
- ⛓️ **Shell Native**: Pipes, chaining (`&&`), and environment variables work as expected.
- 🎭 **Flexible Identity**: Rename the binary to `do`, `run`, or `task` and it adapts automatically.
- ⏱️ **Performance Metrics**: Optional timing for your heavy build scripts.

## Installation

### Homebrew (macOS)

```bash
brew tap mmiraly/tap
brew install cawa
```

### Manual / Build from Source

If you have Rust installed:

```bash
cargo install --path .
```

Or build manually:

```bash
cargo build --release
sudo cp target/release/cs /usr/local/bin/
```

## Usage

### 1. Defining Workflows

```bash
# Define a 'ship' workflow
cs add ship "cargo fmt && cargo test && git push"

# Create a 'wip' checkpoint
cs add wip "git add . && git commit -m 'wip'"

# Run multiple test suites in parallel
cs add -p quality "cargo test --lib" "npm run test:e2e"
```

### 2. Running Workflows

```bash
# Just run it
cs ship

# Pass arguments (passed through to the underlying command)
cs ship -- --force
```

### 3. Management

```bash
cs list
cs remove ship
```

## Configuration

The config lives in `.cawa_cfg.json`. It is meaningful commit this file to git so your team shares the same aliases!

```json
{
  "enable_timing": true,
  "aliases": {
    "release": "./scripts/release.sh"
  }
}
```

## Contributing

We welcome contributions!

1. Fork the repo.
2. Create feature branch (`git checkout -b feature/cool-thing`).
3. Commit changes (`git commit -m 'Add cool thing'`).
4. Push to branch.
5. Open a Pull Request.

### Compiling Locally

```bash
git clone https://github.com/mmiraly/cawa.git
cd cawa
cargo build
./target/debug/cs --help
```

## License

Copyright (C) 2026

This program is free software: you can redistribute it and/or modify
it under the terms of the **GNU General Public License as published by
the Free Software Foundation**, either version 3 of the License, or
(at your option) any later version.

See [LICENSE](LICENSE) for details.

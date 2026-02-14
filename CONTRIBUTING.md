# Contributing to Agora

Thank you for your interest in contributing to Agora! This document will help you get started.

## Development Setup

### Prerequisites

- **Rust** 1.70+ (`rustup`)
- **Node.js** 18+ (for Tauri)
- **Flutter** 3.0+ (for Mobile/Web)
- **Linux**: See dependencies in README.md

### Clone & Build

```bash
git clone https://github.com/your-org/agora.git
cd agora
cargo build
cargo test
```

## Project Structure

```
core/       # Rust core library (networking, crypto, audio)
node/       # Headless node server
cli/        # CLI testing tool
desktop/    # Tauri desktop app
mobile/     # Flutter mobile/web app
```

## Development Workflow

We follow **Shape Up** methodology with 6-week cycles.

### 1. Find an Issue

- Check [Good First Issues](../../issues?q=is%3Aissue+is%3Aopen+label%3A"good+first+issue")
- Read the issue description carefully
- Comment if you want to work on it

### 2. Create a Branch

```bash
git checkout -b feature/issue-123-short-description
```

### 3. Make Changes

- Follow existing code style
- Add tests for new functionality
- Update documentation if needed

### 4. Run Checks

```bash
# Format
cargo fmt

# Lint
cargo clippy -- -D warnings

# Test
cargo test

# Flutter tests (if touching mobile/)
cd mobile && flutter test
```

### 5. Submit PR

- Push your branch
- Create a Pull Request
- Fill out the PR template
- Wait for CI checks and review

## Code Conventions

### Rust

```rust
// Use thiserror for errors
#[derive(Error, Debug)]
pub enum Error {
    #[error("Description: {0}")]
    Variant(String),
}

// Async functions return Result<T>
pub fn do_something() -> Result<String> {
    // Use ? for error propagation
}

// Tests in same file
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_something() {
        // ...
    }
}
```

### Naming

| Type | Convention | Example |
|------|------------|---------|
| Types | PascalCase | `NetworkNode`, `RoomConfig` |
| Functions | snake_case | `generate_room_id` |
| Constants | SCREAMING_SNAKE_CASE | `MAX_PARTICIPANTS` |

### Commits

```
type(scope): short description

- feat(core): add new encryption method
- fix(node): resolve memory leak
- docs(readme): update installation steps
- test(audio): add benchmark tests
```

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                        Clients                               │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │
│  │ Desktop  │  │ Mobile   │  │   Web    │  │   CLI    │    │
│  │ (Tauri)  │  │ (Flutter)│  │ (WebRTC) │  │  (Rust)  │    │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘    │
└───────┼─────────────┼─────────────┼─────────────┼───────────┘
        │             │             │             │
        └─────────────┼─────────────┼─────────────┘
                      │             │
┌─────────────────────┼─────────────┼─────────────────────────┐
│                     │   Core     │                          │
│  ┌──────────────────┼────────────┼──────────────────────┐   │
│  │                  │            │                      │   │
│  │  ┌───────────┐  ┌┴────────┐  ┌┴────────┐  ┌───────┐ │   │
│  │  │  Network  │  │  Audio  │  │  Crypto │  │ Room  │ │   │
│  │  │ (libp2p)  │  │ (Opus)  │  │(ChaCha) │  │  DHT  │ │   │
│  │  └───────────┘  └─────────┘  └─────────┘  └───────┘ │   │
│  └──────────────────────────────────────────────────────┘   │
│                              │                              │
│  ┌───────────────────────────┼──────────────────────────┐   │
│  │                    Node (Headless)                    │   │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐ │   │
│  │  │ Signaling│  │ Dashboard│  │ Metrics │  │Discovery│ │   │
│  │  │   (WS)   │  │  (HTTP)  │  │ (Prom)  │  │  (DHT)  │ │   │
│  │  └─────────┘  └─────────┘  └─────────┘  └─────────┘ │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## RFC Process

For significant changes, create an RFC:

1. Copy `docs/rfc-template.md` to `docs/rfc/0001-your-feature.md`
2. Fill out the template
3. Submit as a PR with `[RFC]` prefix
4. Discuss with maintainers
5. Once approved, implement

## Getting Help

- Open a [Discussion](../../discussions) for questions
- Join our community (link in README)
- Check existing issues before creating new ones

## License

By contributing, you agree that your contributions will be licensed under MIT OR Apache-2.0.

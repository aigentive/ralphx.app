> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

# Rust Stable API Safety

## Goal

Prevent compile breaks from unstable std APIs (e.g., `unsigned_is_multiple_of`) on non-nightly toolchains.

## Rules (NON-NEGOTIABLE)

- Default to stable Rust APIs only. ❌ Unstable std methods/features unless user explicitly asks for nightly-only work.
- For divisibility checks, use `%` form. ✅ `x % n == 0` (with zero guard where needed) | ❌ `x.is_multiple_of(n)`.
- If unstable API is truly required, gate it explicitly and document nightly requirement in PR/commit notes.
- Treat `rust-toolchain.toml` as source of truth for project toolchain expectations.

```rust
// ✅ Stable and portable
if interval != 0 && current_iteration > 0 && current_iteration % interval == 0 {
    // checkpoint
}

// ❌ Do not use (can break on stable toolchains)
if current_iteration.is_multiple_of(interval) { /* ... */ }
```

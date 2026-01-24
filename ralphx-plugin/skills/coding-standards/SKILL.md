---
name: coding-standards
description: Project coding standards and patterns
disable-model-invocation: true
user-invocable: false
---

# Coding Standards

## TypeScript

### General
- Use `strict` mode (all strict flags enabled)
- Prefer `const` over `let`
- Use explicit return types on functions
- No `any` type - use `unknown` if type is truly unknown

### Imports
- Use absolute imports via `@/` alias
- Group imports: external, internal, types
- Use `type` imports for type-only imports

### Naming
- PascalCase for types, interfaces, components
- camelCase for variables, functions, methods
- UPPER_CASE for constants
- Descriptive names over abbreviations

## React

### Components
- Functional components only
- Props interface above component definition
- Destructure props in function signature
- Keep components under 100 lines

### Hooks
- Use hooks for all state management
- Extract complex logic to custom hooks
- Keep hooks under 100 lines
- Name custom hooks with `use` prefix

### Patterns
```tsx
interface Props {
  value: string;
  onChange: (value: string) => void;
}

export function MyComponent({ value, onChange }: Props) {
  return <div>{value}</div>;
}
```

## Rust

### General
- Use `cargo clippy` for linting
- Document public APIs with `///`
- Handle all errors explicitly (no unwrap in production)
- Prefer owned types over references when ownership is clear

### Naming
- snake_case for functions, variables, modules
- PascalCase for types, traits, enums
- UPPER_CASE for constants

### Patterns
- Use `thiserror` for custom errors
- Use `serde` for serialization
- Newtype pattern for type safety

## Testing

### Location
- Test file next to source: `Component.test.tsx`
- Integration tests in `tests/` directory

### Patterns
- Arrange-Act-Assert structure
- One assertion per test when practical
- Descriptive test names: `test_should_reject_invalid_input`

### Tools
- TypeScript: Vitest + React Testing Library
- Rust: built-in `#[test]` + mockall

## File Size Limits

| File Type | Max Lines |
|-----------|-----------|
| Component | 100 |
| Hook | 100 |
| Store | 150 |
| Skill | 150 |
| Agent | 100 |

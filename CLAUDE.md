# samskara-codegen

CozoDB schema → Cap'n Proto → Rust codegen pipeline.

## Purpose

Reads CozoDB `::columns` and `::relations` metadata to deterministically
generate `.capnp` schema files. Cap'n Proto's `capnpc` then compiles these
into typed Rust Reader/Builder types.

## Dependency Position

```
criome-cozo (leaf)
    ↑
samskara-codegen (this crate)
    ↑
samskara-lojix-contract
    ↑
samskara / lojix
```

## Naming Conventions

- Relation `thought` → struct `Thought`
- Relation `agent_session` → struct `AgentSession`
- Column `created_ts` → field `createdTs`
- Vocab `liveness_vocab` → enum `Liveness`

## Deterministic Ordering

- Structs: sorted alphabetically by relation name
- Fields: ordered by `index` from `::columns`
- Enum variants: sorted alphabetically by key value
- File ID (`@0x...`): blake3 of sorted relation names, truncated to u64

## Vocab Detection Rule

A relation is a vocab enum if:
1. Name ends with `_vocab`
2. Exactly one key column
3. Key column type is `String`

## Language Policy

Rust only. No other languages in production paths.

## VCS

Jujutsu (`jj`) is mandatory. Git is the backend only.

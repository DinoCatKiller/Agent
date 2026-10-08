---
id: ResponseFormat
title: ResponseFormat
---

# Enum: ResponseFormat

Defined in: [`crates/common/src/completion.rs:44`](../../../../crates/common/src/completion.rs#L44)

结构化输出请求。

## Variants

### Text

Defined in: [`crates/common/src/completion.rs:45`](../../../../crates/common/src/completion.rs#L45)


***

### JsonObject

Defined in: [`crates/common/src/completion.rs:47`](../../../../crates/common/src/completion.rs#L47)

只保证合法 JSON，不保证 schema 一致（DeepSeek Chat 仅有此档）。


***

### JsonSchema

```rust
{ .. }
```

Defined in: [`crates/common/src/completion.rs:49`](../../../../crates/common/src/completion.rs#L49)

严格 schema 输出；适配器负责把 schema 清洗成该供应商支持的子集。

## Trait Implementations

- `impl Borrow for ResponseFormat`
- `impl BorrowMut for ResponseFormat`
- `impl CloneToUninit for ResponseFormat`
- `impl Into for ResponseFormat`
- `impl From for ResponseFormat`
- `impl TryInto for ResponseFormat`
- `impl TryFrom for ResponseFormat`
- `impl Any for ResponseFormat`
- `impl ToOwned for ResponseFormat`
- `impl DeserializeOwned for ResponseFormat`
- `impl Debug for ResponseFormat`
- `impl Clone for ResponseFormat`
- `impl StructuralPartialEq for ResponseFormat`
- `impl PartialEq for ResponseFormat`
- `impl Serialize for ResponseFormat`
- `impl Deserialize for ResponseFormat`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`


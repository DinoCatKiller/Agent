---
id: StreamEvent
title: StreamEvent
---

# Enum: StreamEvent

Defined in: [`crates/common/src/stream.rs:30`](../../../../crates/common/src/stream.rs#L30)

归一化流式事件。

契约（适配器必须保证）：
1. `Start` 是首个事件，`End` 是末个事件。
2. `Delta` 必须可**无状态拼接**（消费者只做 push_str）。
3. `End` 之前必须给一次 `Usage`。
4. `Error` 之后不得再发任何事件。
5. 截断（EOF 无终止记录）不得静默当成功：补 `End { Truncated }` 或 `Error`。

## Variants

### Start

```rust
{ .. }
```

Defined in: [`crates/common/src/stream.rs:31`](../../../../crates/common/src/stream.rs#L31)


***

### Delta

```rust
{ .. }
```

Defined in: [`crates/common/src/stream.rs:32`](../../../../crates/common/src/stream.rs#L32)


***

### ToolCall

```rust
{ .. }
```

Defined in: [`crates/common/src/stream.rs:33`](../../../../crates/common/src/stream.rs#L33)


***

### Usage

```rust
{ .. }
```

Defined in: [`crates/common/src/stream.rs:34`](../../../../crates/common/src/stream.rs#L34)


***

### Error

```rust
{ .. }
```

Defined in: [`crates/common/src/stream.rs:35`](../../../../crates/common/src/stream.rs#L35)


***

### End

```rust
{ .. }
```

Defined in: [`crates/common/src/stream.rs:36`](../../../../crates/common/src/stream.rs#L36)

## Trait Implementations

- `impl Borrow for StreamEvent`
- `impl BorrowMut for StreamEvent`
- `impl CloneToUninit for StreamEvent`
- `impl Into for StreamEvent`
- `impl From for StreamEvent`
- `impl TryInto for StreamEvent`
- `impl TryFrom for StreamEvent`
- `impl Any for StreamEvent`
- `impl ToOwned for StreamEvent`
- `impl DeserializeOwned for StreamEvent`
- `impl Debug for StreamEvent`
- `impl Clone for StreamEvent`
- `impl StructuralPartialEq for StreamEvent`
- `impl PartialEq for StreamEvent`
- `impl Serialize for StreamEvent`
- `impl Deserialize for StreamEvent`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`


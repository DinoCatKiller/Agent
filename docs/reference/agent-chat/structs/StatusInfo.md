---
id: StatusInfo
title: StatusInfo
---

# Struct: StatusInfo

Defined in: [`crates/features/chat/src/ui.rs:433`](../../../../crates/features/chat/src/ui.rs#L433)

状态栏的宿主信息。

## Fields

### provider

```rust
provider: &'a str
```

Defined in: [`crates/features/chat/src/ui.rs:434`](../../../../crates/features/chat/src/ui.rs#L434)


***

### model

```rust
model: &'a str
```

Defined in: [`crates/features/chat/src/ui.rs:435`](../../../../crates/features/chat/src/ui.rs#L435)


***

### session

```rust
session: Option<&'a str>
```

Defined in: [`crates/features/chat/src/ui.rs:437`](../../../../crates/features/chat/src/ui.rs#L437)

当前会话标题；`None` 表示新会话（首次落盘前）。

## Trait Implementations

- `impl Borrow for StatusInfo`
- `impl BorrowMut for StatusInfo`
- `impl Into for StatusInfo`
- `impl From for StatusInfo`
- `impl TryInto for StatusInfo`
- `impl TryFrom for StatusInfo`
- `impl Any for StatusInfo`
- `impl CastableFrom for StatusInfo`
- `impl CastableFrom for StatusInfo`
- `impl Read for StatusInfo`
- `impl Instrument for StatusInfo`
- `impl WithSubscriber for StatusInfo`
- `impl PolicyExt for StatusInfo`
- `impl IntoEither for StatusInfo`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`


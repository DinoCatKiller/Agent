---
id: Theme
title: Theme
---

# Struct: Theme

Defined in: [`crates/features/chat/src/ui.rs:89`](../../../../crates/features/chat/src/ui.rs#L89)

界面配色（来自 `design/tokens.json`，`D4`）。

## Fields

### bg

```rust
bg: Color
```

Defined in: [`crates/features/chat/src/ui.rs:90`](../../../../crates/features/chat/src/ui.rs#L90)


***

### fg

```rust
fg: Color
```

Defined in: [`crates/features/chat/src/ui.rs:91`](../../../../crates/features/chat/src/ui.rs#L91)


***

### dim

```rust
dim: Color
```

Defined in: [`crates/features/chat/src/ui.rs:92`](../../../../crates/features/chat/src/ui.rs#L92)


***

### accent

```rust
accent: Color
```

Defined in: [`crates/features/chat/src/ui.rs:93`](../../../../crates/features/chat/src/ui.rs#L93)


***

### border

```rust
border: Color
```

Defined in: [`crates/features/chat/src/ui.rs:94`](../../../../crates/features/chat/src/ui.rs#L94)


***

### border_focus

```rust
border_focus: Color
```

Defined in: [`crates/features/chat/src/ui.rs:95`](../../../../crates/features/chat/src/ui.rs#L95)


***

### user

```rust
user: Color
```

Defined in: [`crates/features/chat/src/ui.rs:96`](../../../../crates/features/chat/src/ui.rs#L96)


***

### thinking

```rust
thinking: Color
```

Defined in: [`crates/features/chat/src/ui.rs:97`](../../../../crates/features/chat/src/ui.rs#L97)


***

### tool

```rust
tool: Color
```

Defined in: [`crates/features/chat/src/ui.rs:98`](../../../../crates/features/chat/src/ui.rs#L98)


***

### ok

```rust
ok: Color
```

Defined in: [`crates/features/chat/src/ui.rs:99`](../../../../crates/features/chat/src/ui.rs#L99)


***

### err

```rust
err: Color
```

Defined in: [`crates/features/chat/src/ui.rs:100`](../../../../crates/features/chat/src/ui.rs#L100)


***

### status_bg

```rust
status_bg: Color
```

Defined in: [`crates/features/chat/src/ui.rs:101`](../../../../crates/features/chat/src/ui.rs#L101)


***

### status_fg

```rust
status_fg: Color
```

Defined in: [`crates/features/chat/src/ui.rs:102`](../../../../crates/features/chat/src/ui.rs#L102)

## Implementations

### load()

```rust
pub fn load() -> Self
```

Defined in: [`crates/features/chat/src/ui.rs:129`](../../../../crates/features/chat/src/ui.rs#L129)

编译期内嵌 `design/tokens.json` 并解析；失败回退默认盘（不 panic）。

#### Returns

`Self`

## Trait Implementations

- `impl Borrow for Theme`
- `impl BorrowMut for Theme`
- `impl CloneToUninit for Theme`
- `impl Into for Theme`
- `impl From for Theme`
- `impl TryInto for Theme`
- `impl TryFrom for Theme`
- `impl Any for Theme`
- `impl ToOwned for Theme`
- `impl Equivalent for Theme`
- `impl CastableFrom for Theme`
- `impl CastableFrom for Theme`
- `impl Read for Theme`
- `impl Instrument for Theme`
- `impl WithSubscriber for Theme`
- `impl PolicyExt for Theme`
- `impl IntoEither for Theme`
- `impl Equivalent for Theme`
- `impl Debug for Theme`
- `impl Clone for Theme`
- `impl Copy for Theme`
- `impl StructuralPartialEq for Theme`
- `impl PartialEq for Theme`
- `impl Eq for Theme`
- `impl Default for Theme`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`


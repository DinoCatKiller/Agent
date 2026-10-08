---
id: ContentPart
title: ContentPart
---

# Enum: ContentPart

Defined in: [`crates/common/src/message.rs:23`](../../../../crates/common/src/message.rs#L23)

消息内容块。一期只实现 Text / Image / File，音频留位。

## Variants

### Text

```rust
{ .. }
```

Defined in: [`crates/common/src/message.rs:24`](../../../../crates/common/src/message.rs#L24)


***

### Image

```rust
{ .. }
```

Defined in: [`crates/common/src/message.rs:28`](../../../../crates/common/src/message.rs#L28)

图片：URL 或 `data:` URI（base64）。


***

### File

```rust
{ .. }
```

Defined in: [`crates/common/src/message.rs:34`](../../../../crates/common/src/message.rs#L34)

附件（PDF / 文本等）。一期只透传，不做解析。

## Implementations

### text()

```rust
pub fn text<impl Into<String>: Into>(text: impl ?) -> Self
```

Defined in: [`crates/common/src/message.rs:46`](../../../../crates/common/src/message.rs#L46)

#### Parameters

##### text

`impl ?`

#### Returns

`Self`

## Trait Implementations

- `impl Borrow for ContentPart`
- `impl BorrowMut for ContentPart`
- `impl CloneToUninit for ContentPart`
- `impl Into for ContentPart`
- `impl From for ContentPart`
- `impl TryInto for ContentPart`
- `impl TryFrom for ContentPart`
- `impl Any for ContentPart`
- `impl ToOwned for ContentPart`
- `impl DeserializeOwned for ContentPart`
- `impl Debug for ContentPart`
- `impl Clone for ContentPart`
- `impl StructuralPartialEq for ContentPart`
- `impl PartialEq for ContentPart`
- `impl Serialize for ContentPart`
- `impl Deserialize for ContentPart`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`


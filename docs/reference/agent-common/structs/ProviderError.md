---
id: ProviderError
title: ProviderError
---

# Struct: ProviderError

Defined in: [`crates/common/src/error.rs:45`](../../../../crates/common/src/error.rs#L45)

统一错误包装。

## Fields

### provider

```rust
provider: String
```

Defined in: [`crates/common/src/error.rs:46`](../../../../crates/common/src/error.rs#L46)


***

### category

```rust
category: ErrorCategory
```

Defined in: [`crates/common/src/error.rs:47`](../../../../crates/common/src/error.rs#L47)


***

### retryable

```rust
retryable: bool
```

Defined in: [`crates/common/src/error.rs:49`](../../../../crates/common/src/error.rs#L49)

是否可重试。默认取自 `category`，策略层可覆盖。


***

### status

```rust
status: Option<u16>
```

Defined in: [`crates/common/src/error.rs:51`](../../../../crates/common/src/error.rs#L51)


***

### request_id

```rust
request_id: Option<String>
```

Defined in: [`crates/common/src/error.rs:53`](../../../../crates/common/src/error.rs#L53)


***

### raw

```rust
raw: Option<Value>
```

Defined in: [`crates/common/src/error.rs:56`](../../../../crates/common/src/error.rs#L56)

供应商原始错误体，排错与 fixtures 用。


***

### source_message

```rust
source_message: Option<String>
```

Defined in: [`crates/common/src/error.rs:59`](../../../../crates/common/src/error.rs#L59)

底层原因（已字符串化，保证可序列化）。

## Implementations

### new()

```rust
pub fn new<impl Into<String>: Into>(provider: impl ?, category: ErrorCategory) -> Self
```

Defined in: [`crates/common/src/error.rs:63`](../../../../crates/common/src/error.rs#L63)

#### Parameters

##### provider

`impl ?`

##### category

[`ErrorCategory`](../enums/ErrorCategory.md)

#### Returns

`Self`


***

### with_status()

```rust
pub fn with_status(self, status: u16) -> Self
```

Defined in: [`crates/common/src/error.rs:75`](../../../../crates/common/src/error.rs#L75)

#### Parameters

##### status

`u16`

#### Returns

`Self`


***

### with_request_id()

```rust
pub fn with_request_id<impl Into<String>: Into>(self, request_id: impl ?) -> Self
```

Defined in: [`crates/common/src/error.rs:80`](../../../../crates/common/src/error.rs#L80)

#### Parameters

##### request_id

`impl ?`

#### Returns

`Self`


***

### with_raw()

```rust
pub fn with_raw(self, raw: Value) -> Self
```

Defined in: [`crates/common/src/error.rs:85`](../../../../crates/common/src/error.rs#L85)

#### Parameters

##### raw

`Value`

#### Returns

`Self`


***

### with_message()

```rust
pub fn with_message<impl Into<String>: Into>(self, message: impl ?) -> Self
```

Defined in: [`crates/common/src/error.rs:90`](../../../../crates/common/src/error.rs#L90)

#### Parameters

##### message

`impl ?`

#### Returns

`Self`


***

### retryable_override()

```rust
pub fn retryable_override(self, retryable: bool) -> Self
```

Defined in: [`crates/common/src/error.rs:96`](../../../../crates/common/src/error.rs#L96)

覆盖默认重试判定（例如业务判定某类 429 不该重试）。

#### Parameters

##### retryable

`bool`

#### Returns

`Self`

## Trait Implementations

- `impl Borrow for ProviderError`
- `impl BorrowMut for ProviderError`
- `impl CloneToUninit for ProviderError`
- `impl Into for ProviderError`
- `impl From for ProviderError`
- `impl TryInto for ProviderError`
- `impl TryFrom for ProviderError`
- `impl Any for ProviderError`
- `impl ToOwned for ProviderError`
- `impl ToString for ProviderError`
- `impl DeserializeOwned for ProviderError`
- `impl Debug for ProviderError`
- `impl Clone for ProviderError`
- `impl StructuralPartialEq for ProviderError`
- `impl PartialEq for ProviderError`
- `impl Serialize for ProviderError`
- `impl Deserialize for ProviderError`
- `impl Display for ProviderError`
- `impl Error for ProviderError`

## Auto Trait Implementations

`Freeze` `RefUnwindSafe` `Send` `Sync` `Unpin` `UnsafeUnpin` `UnwindSafe`


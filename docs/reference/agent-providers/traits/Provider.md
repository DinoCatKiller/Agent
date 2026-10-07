---
id: Provider
title: Provider
---

# Trait: Provider

Defined in: [`crates/kernel/providers/src/lib.rs:63`](../../../../crates/kernel/providers/src/lib.rs#L63)

供应商适配器契约。

- `chat` 与 `stream` 都必须实现（见 A2 §6 落地清单）。
- `stream` **同步返回**流句柄：鉴权/参数错误要在发出请求前就暴露，
  而不是等消费者第一次 poll。
- 厂商特有参数走 `ModelRequest::additional_params`，不得污染公共字段。
- `Clone` 是硬要求：适配器内部用 `Arc` 持有 client，clone 只是引用计数，
  类型擦除（[`ErasedProvider`](ErasedProvider.md)）需要 owned future，见下方实现。


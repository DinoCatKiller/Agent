# 11 · Provider 接入契约

> 索引 ID：`A2` ｜ 状态：🚧 草案 ｜ 最后更新：2026-09-27

## 目的

定义所有供应商适配器必须实现的统一接口。新增供应商 = 实现本契约 + 注册表登记，核心代码零改动。

## 适用范围

- 包含：能力协商、请求/响应、流式事件、错误分类的归一化约定。
- 不包含：厂商字段差异（`providers/*.md`）、路由与兜底策略（`21-model-registry.md`）。

## 1. 能力协商

```ts
type Capability =
  | 'text' | 'vision' | 'audio' | 'file'
  | 'tools' | 'json_mode' | 'reasoning' | 'logprobs' | 'embedding';

interface ModelSpec {
  id: string;
  provider: string;
  contextWindow: number;
  maxOutput: number;
  capabilities: Set<Capability>;
  pricing?: { inputPerMTok: number; outputPerMTok: number; currency: string };
  deprecated?: boolean;
}
```

规则：请求所需能力必须是模型能力的子集，否则在**路由阶段 fast-fail**，不要发到供应商再报错。

## 2. 统一接口（伪代码，语言随 ADR-001 定）

```ts
interface ModelProvider {
  readonly id: string;
  listModels(): Promise<ModelSpec[]>;
  chat(req: ModelRequest, opt?: CallOptions): Promise<ModelResponse>;
  stream(req: ModelRequest, opt?: CallOptions): AsyncIterable<StreamEvent>;
  countTokens?(req: ModelRequest): Promise<number>;
  close?(): Promise<void>;
}
```

核心只用上面这些方法。厂商特有参数一律走 `req.providerOptions[providerId]`，**不允许污染公共字段**。

## 3. 请求 / 响应

```ts
interface ModelRequest {
  model: string;
  messages: Message[];
  temperature?: number;
  maxTokens?: number;
  tools?: ToolDef[];
  toolChoice?: ToolChoice;
  responseFormat?: { type: 'text' | 'json_object' | 'json_schema'; schema?: unknown };
  stop?: string[];
  signal?: AbortSignal;
  metadata?: Record<string, unknown>;
  providerOptions?: Record<string, unknown>;
}

interface ModelResponse {
  id: string;
  model: string;
  text: string;
  toolCalls?: ToolCall[];
  finishReason: 'stop' | 'length' | 'tool_calls' | 'content_filter' | 'error';
  usage: { inputTokens: number; outputTokens: number; totalTokens: number };
  raw?: unknown;
}
```

## 4. 流式事件（归一化）

| 事件 | 载荷 | 说明 |
|------|------|------|
| `start` | `{ responseId, model }` | 首个事件 |
| `delta` | `{ type: 'text' \| 'thinking' \| 'tool_args', text }` | 增量，必须可无状态拼接 |
| `tool_call` | `{ call }` | 完整工具调用 |
| `usage` | `{ usage }` | 结束前必须给一次 |
| `error` | `{ error }` | 之后不再发任何事件 |
| `end` | `{ finishReason }` | 末个事件 |

## 5. 错误分类

| 类别 | 触发 | 默认策略 |
|------|------|---------|
| `auth` | 401 / 403 / 密钥无效 | 不重试，告警 |
| `rate_limit` | 429 | 指数退避 + 抖动，必要时换模型 |
| `timeout` | 超时 | 短请求重试，长请求改流式 |
| `invalid_request` | 400（参数/模型不支持） | 不重试，降级到兼容模型 |
| `server` | 5xx | 退避重试 |
| `context_overflow` | 超上下文窗口 | 裁剪/摘要后重试一次 |
| `content_filter` | 命中审核 | 不重试，上抛业务 |
| `unknown` | 其它 | 保留 raw 后上抛 |

统一包装：

```ts
class ProviderError extends Error {
  provider: string;
  category: ErrorCategory;
  retryable: boolean;
  status?: number;
  requestId?: string;
  raw?: unknown;
}
```

## 6. 新增供应商落地清单

- [ ] 建 `docs/providers/<name>.md`：endpoint、鉴权、字段差异、配额、已知坑
- [ ] 实现 `ModelProvider`（`chat` 与 `stream` 都要）
- [ ] 声明 `ModelSpec` 清单：上下文窗口、能力、价格
- [ ] 错误映射覆盖第 5 节全部类别
- [ ] 契约测试：用同一份 fixtures 跑通（见 `30-testing.md`）
- [ ] 密钥走配置层，禁止写死（见 `20-config-and-secrets.md`）
- [ ] 在 `AGENTS.md` 索引中回填状态为 ✅

## 7. 待定问题

- embedding / rerank 是否纳入同一契约
- 是否优先只接 OpenAI 兼容协议以降低成本
- `countTokens` 是否强制实现

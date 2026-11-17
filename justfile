# CodingRocket 统一命令入口：`just --list` 看全部。
# 依赖：just、rustup（stable；`docs` 另需 nightly 与 python）。
# 详见 `Q1` §5（门禁）与 `crates/app/README.md`（运行方式）。

# 包名（内部技术名）；产物可执行文件名是 codingrocket（`S2` Q7）
app := "agent-app"
python := "python"

# 默认：列出所有命令
default:
    @just --list

# ---------- 门禁 ----------

# 一键门禁（提交前跑这个）：格式 + clippy + 测试 + 契约冒烟（`Q1` §5）
check: fmt-check lint test smoke

alias ci := check

# 格式检查（不改文件）
fmt-check:
    cargo fmt --all -- --check

# 自动格式化
fmt:
    cargo fmt --all

# 静态检查（`Q1` §5：`--workspace` 与 `--all-features` 都不能省）
lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# 全部测试
test:
    cargo test --workspace

# 需要真实系统钥匙串的测试（默认 #[ignore]；`R1`）
test-keyring:
    cargo test -p agent-config -- --ignored

# 契约层冒烟（断网，M1）
smoke:
    cargo run -p {{app}} -- self-check

# ---------- 构建与运行 ----------

# 构建整个 workspace
build:
    cargo build --workspace

# 构建二期 GUI（`--features gui`，默认构建不含 GPUI）
build-gui:
    cargo build -p {{app}} --features gui

# 发布构建
build-release:
    cargo build --workspace --release

# CLI 帮助 / 任意子命令透传：`just run -- --help`、`just run -- chat`
run *ARGS:
    cargo run -p {{app}} -- {{ARGS}}

# 对话 REPL（M4）
chat *ARGS:
    cargo run -p {{app}} -- chat {{ARGS}}

# 一期 TUI（M5）
tui *ARGS:
    cargo run -p {{app}} -- tui {{ARGS}}

# 二期 GUI（`D3` 方案 A）：`just gui --provider openai-compatible --model gpt-4o`
gui *ARGS:
    cargo run -p {{app}} --features gui -- gui {{ARGS}}

# ---------- 文档 ----------

# 重新生成 API 参考（docs/reference/，需 nightly + python）
docs:
    {{python}} scripts/generate-docs.py --all

# 只生成一个 crate：`just docs-crate agent-app`
docs-crate crate:
    {{python}} scripts/generate-docs.py --crate {{crate}}

# 复用已有 rustdoc JSON（不重跑 nightly）
docs-no-build:
    {{python}} scripts/generate-docs.py --all --no-build

# ---------- 其他 ----------

# 清理构建产物
clean:
    cargo clean

# 一键提 PR。中文消息先写进 UTF-8 文件再传路径，避免命令行乱码：
#   just pr tmp/msg.txt   →   ./pr.ps1 -msgFile tmp/msg.txt
pr msgfile:
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File pr.ps1 -msgFile "{{msgfile}}"

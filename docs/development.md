# 开发指南

[文档首页](../README.zh-CN.md) · [使用指南](guide.md) · [架构设计](architecture.md)

以下命令均在项目根目录执行。

## 完整检查

需要 Rust 1.85+、Node.js 22+、`rustfmt`、Clippy，以及当前工具链的 `wasm32-unknown-unknown` 目标。检查使用离线模式，无 npm 依赖。

```text
node scripts/check.cjs
```

流程依次执行格式检查、Clippy、Rust 测试、doctest、Rustdoc、Node 测试、wasm release 构建与产物验证，以及本地打包。Clippy 和 Rustdoc 将警告视为错误，任一步失败即停止；打包不上传产物。选项见 `node scripts/check.cjs --help`。

安装好 Rust 1.85.0 及其 wasm 目标后，验证最低版本兼容性：

```text
node scripts/check.cjs --toolchain 1.85.0 --compatibility
```

`--compatibility` 仅跳过格式和 Clippy；`--toolchain` 同时约束 Cargo、Rustdoc 及测试直接调用的 `rustc`。[CI](../.github/workflows/ci.yml) 在 Windows、Linux 上检查 stable，并在 Linux 上验证 1.85.0。

## 单项检查

| 检查 | 命令 |
| --- | --- |
| Rust 测试 | `cargo test --offline --all-targets` |
| doctest | `cargo test --offline --doc` |
| wasm 验证器 | `node --test tests/verify-wasm.cjs` |
| 缓冲区示例 | `node --test tests/doc-buffer.cjs` |
| wasm 示例构建 | `cargo build --offline --release --target wasm32-unknown-unknown --example guest` |
| wasm 产物验证 | `node examples/verify-wasm.cjs target/wasm32-unknown-unknown/release/examples/guest.wasm` |
| API 文档 | `cargo doc --offline --no-deps --open` |
| 发布包 | `cargo package --allow-dirty --offline` |

`--all-targets` 不包含 doctest，两类测试需要分别执行。缓冲区测试需要 wasm 目标；验证器自测只需 Node.js。

上述路径使用默认产物目录；设置 `CARGO_TARGET_DIR` 后，应同步调整 wasm 路径。API 文档默认位于 `target/doc/abi_adapter/index.html`。

## 验证范围

| 改动 | 重点验证 |
| --- | --- |
| 转换 trait 或内置实现 | `src/impls.rs` 单元测试、`tests/behavior.rs`：位内容、槽宽度及单向实现 |
| 宏生成与执行语义 | `tests/behavior.rs`：转换顺序、调用次数、零槽编码、上下文和 host panic 清理 |
| 匹配、诊断与名称解析 | `tests/compile.rs`、`tests/regressions.rs`、`tests/forbid-unsafe.rs`：错误原因、条件编译、跨 crate 与导出属性 |
| wasm 边界与使用方协议 | `examples/guest.rs` 及其验证器、`tests/verify-wasm.cjs`、`tests/doc-buffer.cjs`：实际签名、trap 与内存写入范围 |

`compile_fail` 只证明编译失败；`tests/compile.rs` 还检查错误原因并编译修正对照。wasm 验证器用明确的 `i32` 签名检查实际产物，本机 Rust 调用和 JavaScript 数值结果不能替代此检查。

## 文档维护

README 保留接入示例与导航，使用指南集中常见用法和问题，Rustdoc 维护精确接口契约。架构文档独立说明结构、设计理由和关键约束，开发命令集中在本页。

接口变更时同步更新对应契约、示例与测试；双语 README 保持一致。普通注释及文档示例中的代码注释使用英文。`tests/doc-buffer.cjs` 会直接提取[缓冲区示例](buffers.md)中的 Rust 代码，移动示例时须同步调整测试。

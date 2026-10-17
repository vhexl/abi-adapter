# abi-adapter

[![CI](https://github.com/vhexl/abi-adapter/actions/workflows/ci.yml/badge.svg)](https://github.com/vhexl/abi-adapter/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/vhexl/abi-adapter/branch/main/graph/badge.svg)](https://codecov.io/gh/vhexl/abi-adapter)
[![crates.io](https://img.shields.io/crates/v/abi-adapter.svg)](https://crates.io/crates/abi-adapter)
[![docs.rs](https://img.shields.io/docsrs/abi-adapter)](https://docs.rs/abi-adapter)
[![MSRV](https://img.shields.io/crates/msrv/abi-adapter)](https://github.com/vhexl/abi-adapter/blob/main/Cargo.toml)
[![License](https://img.shields.io/crates/l/abi-adapter.svg)](#许可证)

[English](README.md) | 简体中文

为普通 Rust 函数生成 wasm 调用接口。`abi!` 按声明转换参数、调用目标函数，再转换返回值。
无第三方依赖，支持 `no_std`，需要 Rust 1.85+。

## 快速开始

添加依赖：

```text
cargo add abi-adapter
```

如需使用 `main` 分支尚未发布的改动，改为依赖 Git 仓库：

```text
cargo add abi-adapter --git https://github.com/vhexl/abi-adapter
```

```rust
use abi_adapter::abi;

fn double(value: u32) -> u32 {
    value * 2
}

abi! { host pub fn double_abi([value]) -> 1 => crate::double; }

fn main() {
    assert_eq!(double_abi(21), 42);
}
```

`host` 生成普通 Rust 函数；`guest("double.v1")` 生成 wasm 导出。
每个方括号组对应一个目标参数。槽在 Rust 侧为 `u32`，wasm 侧为 `i32`，包装函数仅直接返回零或一个槽。
自定义类型通过 `FromSlots<N>` 和 `IntoSlots<N>` 转换。
完整 wasm 示例见 [examples/guest.rs](examples/guest.rs)。

运行时注册、内存协议和错误码由使用方定义。转换与目标函数的 `panic` 不会被捕获；guest 导出中表现为 wasm trap。

## 文档导航

- [使用指南](docs/guide.md)：自定义转换、`ctx`、wasm 导出与常见问题。
- [缓冲区示例](docs/buffers.md)：多槽结果与内存约定。
- [架构设计](docs/architecture.md)：模块划分、执行流程与设计取舍。

## 开发

`node scripts/check.cjs` 执行完整的本地检查：格式、Clippy、测试、Rustdoc、wasm 构建与验证，以及打包。
需要 Rust 1.85+（含 `rustfmt`、Clippy 和 `wasm32-unknown-unknown` 目标）与 Node.js 22+。
验证最低支持版本时，安装带同一目标的 Rust 1.85.0，再运行
`node scripts/check.cjs --toolchain 1.85.0 --compatibility`。

## 发布流程

本项目使用 [release-plz](https://release-plz.dev/) 自动化发布：

1. 每次推送到 `main` 分支，release-plz 会自动创建/更新一个 **Release PR**，
   包含版本号与基于 [约定式提交](https://www.conventionalcommits.org/) 生成的更新日志。
2. 合并 Release PR 后，release-plz 将自动：
   - 创建 git 标签（`v<version>`）
   - 发布到 [crates.io](https://crates.io/crates/abi-adapter)
   - 创建 GitHub Release 及发布说明

发布使用 crates.io 的 [Trusted Publishing](https://crates.io/docs/trusted-publishing)，不保存发布令牌；
配置说明见 [release-plz.yml](.github/workflows/release-plz.yml)。

## 许可证

你可以任选以下任一许可证：

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

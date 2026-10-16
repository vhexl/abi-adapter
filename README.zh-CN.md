# abi-adapter

[English](README.md) | 简体中文

为普通 Rust 函数生成 wasm 调用接口。`abi!` 按声明转换参数、调用目标函数，再转换返回值。
无第三方依赖，支持 `no_std`，需要 Rust 1.85+。

## 快速开始

通过 Git 依赖引入：

```toml
[dependencies]
abi-adapter = { git = "https://github.com/vhexl/abi-adapter" }
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
- [开发指南](docs/development.md)：检查、API 文档与维护约定。

## 许可证

[MIT](LICENSE-MIT) 或 [Apache-2.0](LICENSE-APACHE)，任选其一。

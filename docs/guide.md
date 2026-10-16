# 使用指南

[文档首页](../README.zh-CN.md) · [架构设计](architecture.md) · [开发指南](development.md)

`abi!` 将以 `u32` 为槽的接口转换为普通 Rust 函数调用。本页介绍常见用法和排错；
完整声明规则见 [`abi!` API](../src/boundary.rs)，转换契约见 [转换 trait API](../src/slots.rs)。

## 参数与返回值

每个非空方括号组对应业务函数的一个参数，组内名字依次成为包装函数的槽参数：

```text
business: score(point: Point, scale: u32)
groups:   [x, y], [scale]
wrapper:  score_abi(x: u32, y: u32, scale: u32)
```

宏按从左到右的顺序转换参数，调用业务函数一次，再转换返回值。
业务类型由函数签名推断；无参数时写 `fn name()`。

内置转换支持 `()`、`u32`、`i32`、`f32` 和 `[u32; N]`。
整数保留原始位，浮点数使用 `to_bits` / `from_bits`；传递 `1.0f32` 应使用 `1.0f32.to_bits()`。

返回宽度必须写字面量 `-> 0` 或 `-> 1`，分别要求业务返回类型实现
`IntoSlots<0>` 或 `IntoSlots<1>`，包装函数返回 `()` 或 `u32`。
`-> 0` 仍执行返回转换，不能用于忽略 `u32` 返回值；宽度本身不表示成功或失败。
多槽结果需自行约定缓冲区或资源编号协议，宏不会分配或写入结果内存，见[缓冲区示例](buffers.md)。

## 传入自定义类型

参数实现 `FromSlots<N>`，返回值实现 `IntoSlots<N>`；只需实现使用的方向：

```rust
use abi_adapter::{abi, FromSlots, IntoSlots};

struct Point { x: u32, y: u32 }
struct Score(u32);

impl FromSlots<2> for Point {
    fn from_slots([x, y]: [u32; 2]) -> Self {
        Self { x, y }
    }
}

impl IntoSlots<1> for Score {
    fn into_slots(self) -> [u32; 1] {
        [self.0]
    }
}

fn score(point: Point, scale: u32) -> Score {
    Score((point.x + 10 * point.y) * scale)
}

abi! { host fn score_abi([x, y], [scale]) -> 1 => crate::score; }
```

`score_abi(2, 3, 4)` 返回 `128`。`bool`、枚举、`u64`、`f64` 等的表示由使用方定义；
为已有类型选择其他表示时，应定义自己的包装类型。

转换接口不返回 `Result`，实现必须对任意输入位模式保持内存安全。
需要校验的数据可保留到业务函数中处理，并按双方约定返回错误状态；panic 行为见[常见问题](#常见问题)。

## 使用宿主状态

`host` 的 `ctx` 位于槽分组前，以分号分隔，不占槽；它原样传入业务函数首参：

```rust
use abi_adapter::abi;

fn add_total(total: &mut u32, value: u32) -> u32 {
    *total += value;
    *total
}

abi! {
    host fn add_total_abi(ctx total: &mut u32; [value])
        -> 1 => crate::add_total;
}
```

仅有上下文时写 `fn name(ctx state: State;)`。上下文类型原样保留，包括生命周期；
可以使用运行时提供的类型，本库不绑定具体运行时。`guest` 不接受 `ctx`。

## 导出 wasm 函数

`guest("double.v1")` 生成导出名为 `double.v1` 的 `extern "C"` 函数：

```rust
use abi_adapter::abi;

fn double(value: u32) -> u32 { value * 2 }
abi! { guest("double.v1") pub fn double_abi([value]) -> 1 => crate::double; }
```

将使用方的库目标设为 `cdylib`，再构建 wasm：

```toml
[lib]
crate-type = ["cdylib"]
```

```text
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
```

默认产物目录为 `target/wasm32-unknown-unknown/release/`。
本例的 wasm 签名为 `(i32) -> i32`；宿主按 `double.v1` 调用，Rust 函数名仍是 `double_abi`。
导出名必须在最终产物中唯一，调用签名必须兼容；不能依赖链接器发现所有符号冲突。
这些责任也适用于没有 `unsafe` 调用的 Rust 包装函数。

可为同一业务函数声明多个版本的导出。本项目的 wasm 示例与验证命令见[开发指南](development.md)。

## 约定调用方式

调用双方需共同确定：

| 内容 | 示例 |
| --- | --- |
| 导出名和版本 | `score.v1` |
| 槽顺序、分组与表示 | 前两槽组成坐标，第三槽是倍率；浮点数传位模式 |
| 返回与错误协议 | 一槽结果、状态码加结果缓冲区，或资源编号 |
| 内存与资源规则 | 容量、字节序、有效期、所有权及释放责任 |

所有槽的 wasm 类型均为 `i32`；槽数相同时，类型检查无法发现顺序或含义错误。
应检查最终导出表及签名，并通过实际调用验证协议；host 包装还需由运行时注册为相应导入。
JavaScript 将最高位为 1 的 `i32` 结果解释为负数；协议使用 `u32` 时可用 `result >>> 0`。

## 常见问题

| 现象 | 处理方式 |
| --- | --- |
| 声明被拒绝 | 明确写 `-> 0` / `-> 1` 和末尾分号；槽分组不能为空，`guest` 无 `ctx`；只接受同步、安全函数声明 |
| E0061 / E0277 | 每组对应一个业务参数；检查类型的 `FromSlots<N>` 和返回值的 `IntoSlots<0>` / `IntoSlots<1>` |
| E0415 / E0005 / E0618 | 槽名必须唯一，避开可见常量、单元结构体和无字段枚举项；目标使用 `crate::function` 等限定路径 |
| E0283：类型无法推断 | 显式写 `crate::function::<u32>`；参数位置的 `impl Trait` 可通过具体类型的转接函数调用 |
| 限定 trait 方法路径被拒绝 | `<Type as Trait>::method` 不受支持；使用无歧义的 `Type::method` 或转接函数 |
| 外层宏转发宽度失败 | 使用 `$width:tt` 捕获并转发，避免 `literal` / `expr` 片段 |
| E0119：转换实现冲突 | 避免重叠的 trait 实现；用自己的包装类型区分表示 |
| 条件编译后找不到目标 | 属性只作用于包装；将业务函数与包装放入同一个带 `#[cfg(...)]` 的模块 |
| `forbid(unsafe_code)` 与 guest | 当前编译器允许外部宏生成的导出属性；不要添加与 `forbid` 冲突的 `#[allow(unsafe_code)]`，该行为不免除导出安全责任 |
| 找不到导出或数据不符 | 核对实际导出名、签名、符号唯一性与[双方协议](#约定调用方式) |

参数转换、业务调用和返回转换中的 panic 均不会被捕获或转为错误码。
`host` 按项目的 Rust panic 策略传播或终止；`guest` 不能跨 `extern "C"` 展开，
会终止程序，在 wasm 上表现为 trap，且不保证此前已转换值的析构。
预期的输入错误应在业务代码中处理；返回 `Result` 不会自动生成错误协议。

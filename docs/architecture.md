# abi adapter 架构模型

> 状态：现行规范（2026-10-01，单 crate 版）。边界声明使用 `abi!` 声明宏；直接返回仅 0/1 槽；
> 无过程宏、无普通依赖。本地历史材料位于 `backup/architecture.attribute-macros.md`、
> `backup/design.md` 与 `backup/abstraction-study.md`；这些材料不随发布包附带，也不作为现行规范。
> 定位：把普通 Rust 函数按显式槽分组声明为 wasm ABI 函数——解码参数、恰好调用一次、
> 按声明宽度返回，不解释业务语义。

## 1. 概述与核心原则

库只提供两样东西：槽转换 trait（`FromSlots` / `IntoSlots`）与 `abi!` 声明宏。被声明引用的
普通函数称**业务函数**；声明生成一个包装函数，业务函数本身不被改动。

五条原则：

1. **机械变换**：包装函数只做三件事——按声明的槽分组解码参数、解码成功后恰好调用业务函数一次、按声明宽度编码返回。
2. **不解释**：不识别 `Result`、错误码或状态；不拆解返回值结构；不添加未声明的参数、返回值或分支。
3. **不预设**：返回宽度只能显式写 `0` 或 `1`；无法用 0/1 表达的结果由消费者协议承载，宏不会替换为项目预设的返回协议。
4. **宽度即宽度**：槽数只描述值与槽的转换宽度。0/1 槽在两个方向上的线上形状一致（`(i32, …) -> () | i32`），宏不生成指针、内存访问或隐藏协议。
5. **协议归消费者**：成功/失败、错误码、缓冲与句柄都是消费者约定；适配器没有错误通道。

## 2. 组件与职责

| 组件 | 职责 | 不承担 |
|---|---|---|
| 库 `abi-adapter`（单 crate） | `FromSlots` / `IntoSlots` 与有限内置实现；`abi!` 边界声明宏 | 运行时类型、错误包装、协议、调用器、注册清单 |
| 消费者 | 业务函数与槽转换实现、边界声明、调用侧代码、协议（如有）、运行时集成 | — |

- 单 crate：宏就在库里（`#[macro_export]` 使 `abi_adapter::abi!` 在任意路径可用）；没有过程宏，
  没有普通依赖（`syn` / `quote` / `proc-macro2` 已移除）。
- `$crate` 定位转换 trait 与槽类型：依赖改名、facade 重导出、库内自用都无需路径配置
  （旧版属性 `core_path` 因此删除）。
- 本项目不绑定任何 wasm 运行时；运行时集成属于消费者。

## 3. 槽与转换

槽是 32 位字：ABI 层使用 wasm `i32`，Rust 侧使用 `u32` 与 `[u32; N]`。有符号值按原始 32 位
内容传递，浮点值按位模式传递（`f32` 为 `to_bits` / `from_bits`）。

```rust
pub trait FromSlots<const N: usize>: Sized {
    fn from_slots(slots: [u32; N]) -> Self;
}
pub trait IntoSlots<const N: usize>: Sized {
    fn into_slots(self) -> [u32; N];
}
```

- 两个 trait 只回答"一个值与 N 个槽如何互转"，不代表直接返回、缓冲或错误等去向；入站与出站类型各自只实现需要的方向。
- 都是安全接口：`from_slots` 必须能处理任意槽位组合；不提供转换失败通道，需要校验时由消费者自行定义。
- 转换实现可能 panic；适配器不捕获、不转换。host 包装照常向调用方传播 panic；guest 包装是 `extern "C"` 函数，panic 不能越过该边界——程序直接终止（wasm 上为 trap），且已解码值在终止前是否被析构未指定。
- 内置实现（有限集合）：`()` 占 0 槽，`u32`、`i32`、`f32` 各占 1 槽，`[u32; N]` 占 N 槽。`bool`、枚举、窄整数、`u64`/`f64` 的槽表示由消费者决定。
- 同一 trait 不能同时有"覆盖所有满足条件类型"的通用实现与具体实现，两者重叠会导致编译错误（E0119）。缺少内置转换时，建议定义包装类型并自行实现 trait。

## 4. 边界声明宏

### 4.1 语法

```text
abi! { host  #[attrs] VIS fn NAME([a, b], [c])         -> 0|1 => PATH; }
abi! { host  #[attrs] VIS fn NAME(ctx CX: CTY; [a, b]) -> 0|1 => PATH; }
abi! { host  #[attrs] VIS fn NAME(ctx CX: CTY;)        -> 0|1 => PATH; }
abi! { guest("SYMBOL") #[attrs] VIS fn NAME([a, b])    -> 0|1 => PATH; }
```

- `host`：包装函数是**普通 Rust 函数**（无 `extern`、无导出属性），由宿主运行时注册为 guest 导入的实现。
- `guest`：包装函数是**安全 `extern "C"` 函数**，以字符串字面量 `SYMBOL` 导出（`#[unsafe(export_name = …)]`）。导出属性的符号安全义务由消费者承担（§6）。
- 每个方括号组对应一个业务参数；组内每个名字生成一个 `::core::primitive::u32` 包装参数；组顺序即线上顺序，组内顺序即槽顺序。
- `#[attrs]`（如 `#[cfg]`、`#[inline]`）与可见性只作用于包装函数。
- `PATH` 接受普通路径、`Type::method` 与 turbofish 特化路径；不接受 `<Type as Trait>::method` 这种 UFCS 写法，需要具体类型的薄包装函数，或无歧义的 `Type::method` 路径。

### 4.2 解码与推断

声明不写业务类型。包装参数按组解码并直接作为业务调用实参，从左到右求值：

```text
PATH($crate::FromSlots::from_slots([a, b]), $crate::FromSlots::from_slots([c]))
```

`from_slots` 的 `Self` 由业务签名推断（调用位置所需的形参类型），const 参数 `N` 由数组字面量
长度推断。返回位置写 `impl IntoSlots<1>` 也可用（编码侧由实际返回值推断 `Self`）；
`[u32; N]` 形式的 const 泛型数组参数能从组宽度推断（实测：`fn f<const N: usize>(values: [u32; N])`
配 `[x, y, z]` 推断出 `N = 3`）。

### 4.3 返回宽度

| 声明 | 生成体 | 返回类型要求 |
|---|---|---|
| `-> 0` | 执行 `IntoSlots<0>` 并丢弃结果；不返回值 | `IntoSlots<0>` |
| `-> 1` | 返回那一个槽值 | `IntoSlots<1>` |

- 宽度只接受字面量 `0` / `1`。宏以一个 token tree（`$ret:tt`）接收宽度；其它单 token 或带定界符的 token tree 由宏报 `compile_error!`（"ABI returns must be 0 or 1 slots"），`-1`、`1 + 1` 等多 token 写法则在规则匹配时拒绝。
- 外层 `macro_rules!` 转发宽度时应以 `$width:tt` 捕获，保留内部规则匹配 `0` / `1` 所需的原始 token。`$width:literal` 转发后成为不透明 fragment，即使内容是 `0` 或 `1`，内部规则也无法按字面 token 匹配而会拒绝。
- 零槽编码仍实际执行一次，不因输出为空而省略。
- 两个方向都只有直接返回一种形状，P1/P3 不再有承载分歧；更大的结果由消费者协议承载（§7）。

### 4.4 上下文（ctx）

`ctx` 是 host 形式的专用语法：必须是第一个参数，不占槽、不参与槽转换，类型原样复制（含
`'_` 省略生命周期），在解码参数之前按值传入业务函数。guest 形式不接受 `ctx`（匹配错误）。
宏不识别任何运行时类型：`Caller<'_, S>`、`FunctionEnvMut<'_, S>`、`StoreContextMut<'_, S>`
等句柄类型由消费者写在声明里。

### 4.5 拒绝与已知限制

编译期拒绝：

| 写法 | 结果 |
|---|---|
| `-> 2` 或其它非 0/1 的单 token tree 宽度 | 宏自带 `compile_error!` |
| `-> -1`、`-> 1 + 1` 等多 token 宽度 | 宏规则不匹配 |
| 外层宏用 `$width:literal` 捕获后转发返回宽度 | 不透明 fragment 无法匹配内部 `0` / `1` 规则；改用 `$width:tt` |
| guest 形式写 `ctx`、空槽组 `[]`、缺返回子句、`async` / `unsafe` 声明 | 宏规则不匹配（Rust 报错，指向出错 token） |
| 组宽度与业务签名不符、缺少 trait 实现 | E0277 |
| 槽名重复 | E0415（参数重复绑定） |
| **槽名与作用域内常量（或单元结构体/枚举变体）同名** | E0005（参数被解析为常量模式）；`macro_rules!` 无法为调用位置的标识符赋予卫生，改槽名或把声明移到无此名字的作用域 |
| 裸目标路径被同名槽参数捕获 | E0618；目标必须写限定路径 |
| `<Type as Trait>::method` 目标路径 | `$target:path` 规则不匹配；用无歧义的 `Type::method` 或具体薄包装函数 |
| 无法推断类型参数的泛型业务函数、参数位 `impl Trait` | E0283（推断不唯一）；用 turbofish 特化（`crate::f::<u32>`）或具体类型薄包装；const 泛型数组的宽度推断见 §4.2 |

其他已知边界：

- 同型槽的语义顺序无法由类型系统检查（§10）。
- `unsafe_code` lint 默认不检查其它 crate 的宏生成的代码：`abi!` 展开出的
  `#[unsafe(export_name = …)]` 不触发该 lint，`#![forbid(unsafe_code)]` 的 crate 可以直接
  使用 guest 形式；手写或本 crate 内宏生成的导出标记仍会计入。不要加 `#[allow(unsafe_code)]`
  ——`forbid` 下那是 E0453。守卫见 `tests/forbid-unsafe.rs`（编译器现行实现，非语言规范；§9）。
- 属性无法从业务函数自动传播到包装函数；两者需要同一条件编译时，把业务函数与声明放进同一 `cfg` 模块。

## 5. 生成的函数

host（等价展开，`$crate` 渲染为 crate 路径）：

```rust
// 声明
abi! { host pub fn tick_abi(ctx cx: Ctx<'_>; [x, y]) -> 1 => crate::business::tick; }

// 展开
pub fn tick_abi(
    cx: Ctx<'_>,
    x: ::core::primitive::u32,
    y: ::core::primitive::u32,
) -> ::core::primitive::u32 {
    ::abi_adapter::IntoSlots::<1>::into_slots(crate::business::tick(
        cx,
        ::abi_adapter::FromSlots::from_slots([x, y]),
    ))[0]
}
```

guest：

```rust
// 声明
abi! { guest("score.v1") pub fn score_abi([x, y], [scale]) -> 1 => crate::business::score; }

// 展开
#[unsafe(export_name = "score.v1")]
pub extern "C" fn score_abi(
    x: ::core::primitive::u32,
    y: ::core::primitive::u32,
    scale: ::core::primitive::u32,
) -> ::core::primitive::u32 {
    ::abi_adapter::IntoSlots::<1>::into_slots(crate::business::score(
        ::abi_adapter::FromSlots::from_slots([x, y]),
        ::abi_adapter::FromSlots::from_slots([scale]),
    ))[0]
}
```

两个函数都是直线代码：逐参数解码 → 调用一次 → 按宽度转出。包装体不生成 `unsafe` 块，
不自行分配或访问消费者缓冲；转换实现和业务函数仍可按消费者需要执行这些操作。
guest 函数本身是安全函数，但 `#[unsafe(export_name = …)]` 带有编译器无法验证的符号安全义务（§6）。

## 6. 命名、可见性与条件编译

- 包装名、可见性、guest 符号名全部由声明写出；宏不自动命名、不继承任何可见性。
- guest 符号可以携带版本或用途（`score.v1`、`score.v2`）；同一个业务函数可以出现在多个包装中，见 `examples/guest.rs`。
- 消费者必须确保导出符号在最终产物的全局符号空间中唯一，并与实际调用者要求的 ABI 一致，避免覆盖运行时或系统符号（如 `malloc`）。符号碰撞可能报告重复定义，也可能替换已有符号而造成错误调用，不能依赖链接器拒绝所有碰撞。`unsafe(export_name)` 表达的正是这些安全义务，详见 [Rust unsafe 属性说明](https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-attributes.html)。
- 同名包装函数在 Rust 层就是重复定义；私有包装（无 `VIS`）可正常调用，是否真正导出以最终 wasm 导出表为准。
- `#[cfg]` / `#[cfg_attr]` 等属性只作用于包装函数；需要"业务与包装同时消失"时，对包含两者的模块写 `cfg`。

## 7. 消费流程与协议归属

基本流程：

1. 双方约定参数分组、返回宽度与槽编码含义（如有协议：状态、缓冲、句柄）。
2. 被调用侧写业务函数并用 `abi!` 声明；host 侧得到普通函数交给运行时注册，guest 侧构建真实 wasm 导出。
3. 调用侧手写声明与调用代码，用 `IntoSlots` 编码参数。
4. 调用后按约定解释返回值。

- 0/1 槽：两侧线上形状一致，`(i32, …) -> i32 | ()`，可逐入口直接配对注册/导入。
- 更大的结果：消费者自定义协议——把地址或句柄声明为普通槽参数（`[buf_addr]`），状态作为普通 0/1 返回值；写入代码与 `unsafe` 都在消费者自己的业务函数体内（§8、§11）。
- 不生成注册清单、调用器、函数描述符；调用侧代码、句柄缓存与部署配对由消费者维护；两侧共享类型也不保证语义一致，相同的 ABI 形状可以有完全不同的解读。

## 8. 内存与安全边界

包装体不生成 `unsafe` 块、不操作消费者指针或缓冲；guest 导出属性的安全义务见 §6。消费者若自行实现缓冲协议，必须满足相应内存访问的安全要求：

- 缓冲是有效可写分配，容量覆盖全部写入，在调用期间存活，无重叠借用或并发写入。
- 槽的字节序、成功与失败时是否写入都属于消费者协议；§11 示例约定小端写入，失败时保留原缓冲。
- 若从整数恢复可写指针，提供方必须从真实分配的可写原始指针导出地址（`expose_provenance`），恢复方用 `with_exposed_provenance_mut` 还原；只取地址（`addr()`）不够。从静态缓冲直接取得写指针应使用 `&raw mut`，`&raw const` 不提供确定的写权限。宿主局部变量的地址不能当作 guest 地址。消费者持有原始指针时，也可验证地址后直接使用该指针，避免整数恢复。[原始指针的写权限要求](https://doc.rust-lang.org/stable/core/ptr/macro.addr_of.html)
- 安全业务函数必须对任意槽输入保持内存安全；恢复 provenance 不会校验地址、容量或借用。只排除地址 0 不能证明其余地址有效；自有保留缓冲的入口应验证缓冲身份，并使用自己持有的有效指针。
- 用 `u32` 表示指针地址的协议假定 32 位指针；64 位指针的 wasm 目标不适用。
- `write_unaligned` 可写入未对齐地址，但仍要求有效可写内存。构造 `&mut [u32]`（如 `from_raw_parts_mut::<u32>`）或执行普通 `u32` 指针写入则必须满足 `u32` 对齐要求；不能用未对齐写入的规则构造未对齐引用。[可变切片构造的安全要求](https://doc.rust-lang.org/stable/core/slice/fn.from_raw_parts_mut.html)
- 运行时的线性内存范围检查不能证明分配容量与借用规则；可增长内存可能移动基址，不要跨调用缓存裸地址。
- 消费者的 `forbid(unsafe_code)` 不能覆盖外部宏的输出；安全结论以实际展开的代码为准（现行宏的展开不含 `unsafe` 块，见 §4.5 的 `unsafe_code` lint 说明）。

## 9. 环境与结构约束

- 单 crate：`no_std`、零普通依赖、零分配、`#![forbid(unsafe_code)]`；宏在库内（`#[macro_export]`）。
- 生成代码：具体类型、静态数组与直接调用；没有描述符、动态分派、注册表或通用容器。
- 外部宏输出与 `unsafe_code` lint 的关系，以及 `forbid` 下
  `#[allow(unsafe_code)]` 的限制（E0453），见 §4.5；守卫见
  `tests/forbid-unsafe.rs`。
- 目标：一级支持为 wasm32 的 core wasm 模块；组件模型不在范围内（导出由 WIT world 决定）。其他目标不保证可导出，生成代码本身与目标无关、仍可编译。
- 宏不检查目标，不因目标报错或改变生成物。
- edition 2024，Rust 1.85 起；运行时及其版本由消费者选择与维护。
- 性能目标：在相同声明约定下，生成实现与手写实现在 release 构建中合并为同一函数（逐字节一致）。

验证命令（wasm 检查需要已安装 `wasm32-unknown-unknown` 目标和 Node.js）：

```text
cargo test --all-targets
cargo test --doc
node --test tests/verify-wasm.cjs
node --test tests/doc-buffer.cjs
cargo build --release --target wasm32-unknown-unknown --example guest
node examples/verify-wasm.cjs target/wasm32-unknown-unknown/release/examples/guest.wasm
```

普通测试与文档测试验证转换、包装行为和编译期拒绝；`--all-targets` 不包含文档测试。
Node 测试用独立 wasm 反例检查验证器本身；真实产物验证通过带明确 `i32` 签名的导入模块链接
检查参数与结果类型，并验证高位槽、条件编译、解码/业务/编码 panic 的 trap 与手写导出函数身份。
`tests/doc-buffer.cjs` 直接提取 §11 的 guest 代码并编译为 wasm，验证合法地址的小端写入，
以及零地址、未对齐地址、缓冲内部/末尾地址、越界地址和其他分配的地址被拒绝且缓冲保持不变。
这些检查需要单独执行，仅通过 native 测试不能证明 wasm 产物符合约定。
最低支持版本应另外用 Rust 1.85 执行相同检查；当前工具链通过不代表最低支持版本已经通过。

## 10. 两侧一致性的可检测性

**已知风险：相同参数和结果个数的两侧语义错配在类型层面不可检测。** 产物在 wasm 边界上的
类型形状为 `(i32, …) -> i32 | ()`。所有槽都是 `i32`，类型检查只能识别槽数：

- 链接与实例化期的类型检查能区分**参数与结果的个数**；本项目的返回宽度只有 0/1，因此返回宽度错配也能检测。只有参数个数与返回宽度都相同的 ABI 函数才在类型上可互换，槽分组与槽的语义仍无法区分。
- 符号名由消费者选择，可以携带版本或用途（比自动命名多一条人工防错手段），但仍不含分组的机器可读描述；不生成清单、描述符或校验函数。
- 对形状相同的语义错配，后果是**静默**：两侧都能编译、能链接、能调用，然后按各自的解读读写同一批槽。

这是「协议归消费者」（§1 原则 5）的直接代价：项目不为一致性提供机制，也不为其负责。可用的兜底只有两条，都由消费者决定。

1. **同源**：把配对的入口放进同一个 workspace，让类型与转换实现只有一处来源——两侧共用同一个契约模块里的业务类型与 `FromSlots`/`IntoSlots` 实现；槽分组本身无法共享，只能靠同一处来源与评审对齐。
2. **握手**：在协议里加一次版本比对。本项目不生成、不校验，宿主加载后自行比对即可：

```rust
// guest crate：消费者手写的导出，与业务导出并列，不是适配器产物
#[unsafe(export_name = "__abi_contract")]
pub extern "C" fn abi_contract() -> u32 { 3 }  // 消费者自定的契约版本
```

握手只能防"拿错插件 / 签错版本"，防不了同版本内的分组漂移——后者只能靠第 1 条或人工评审。

## 11. 非规范性示例契约

> 本节代码**不属于本项目的约定**：适配器不生成、不检查其中的任何部分。它只演示 §7 那条
> "协议归消费者"的路如何落地。运行时以 wasmi 为例，API 名称随运行时版本变化；示例中的
> 协议逻辑（地址参数、状态返回值、缓冲写入）都属于消费者。

示例约定：wire = `(buf_addr: u32, r: u32, g: u32, b: u32) -> u32`。`buf_addr` 指向 guest 自己
线性内存里保留的 `RET_BUF`，容量 3 个字，逐槽小端；status `0` 表示成功，非 0 表示失败，
缓冲保持不变且宿主不读取结果。入口只接受 `__abi_ret_buf` 返回的缓冲起始地址。

本例仅适用于单线程、非共享内存的 wasm32 实例；宿主只在导出调用之间访问缓冲，不能并发访问。
地址只用于身份校验，写入始终使用 guest 自己从 `RET_BUF` 取得的可写指针，不从外部整数恢复指针。

被调用侧（guest crate）——缓冲地址的提供方式同样由消费者决定，这里用一块保留的静态缓冲；
`unsafe` 与协议逻辑都在业务函数里：

```rust
// Rgb 及其槽编码由消费者定义。
pub struct Rgb { r: u32, g: u32, b: u32 }

impl abi_adapter::FromSlots<3> for Rgb {
    fn from_slots([r, g, b]: [u32; 3]) -> Self {
        Self { r, g, b }
    }
}

static mut RET_BUF: [u32; 3] = [0; 3];

/// 消费者手写：把保留缓冲的地址交给宿主。不是适配器产物。
#[unsafe(export_name = "__abi_ret_buf")]
pub extern "C" fn abi_ret_buf() -> u32 {
    (&raw mut RET_BUF).expose_provenance() as u32
}

/// 业务函数：协议与 unsafe 都在这里；声明只负责槽转换。
pub fn set_color(buf_addr: u32, color: Rgb) -> u32 {
    let out = &raw mut RET_BUF;
    if buf_addr == 0 || buf_addr as usize != out.addr() {
        return 1;
    }
    let words = [color.r.to_le(), color.g.to_le(), color.b.to_le()];
    // SAFETY: out 直接来自可写静态数组 RET_BUF，指针有效且对齐，容量恰好 3 槽。
    // 本例的单线程、非共享内存实例中，调用期间没有宿主访问，也没有重入调用或存活引用。
    unsafe { out.write(words); }
    0
}

abi_adapter::abi! {
    guest("set_color.v1") pub fn set_color_abi([buf_addr], [r, g, b])
        -> 1 => crate::set_color;
}
```

调用侧（宿主）：调用后重新取得宿主侧线性内存视图并解码结果（内存增长可能让旧视图失效，§8；guest 静态缓冲的线性内存偏移不会因此改变）：

```rust
let addr = abi_ret_buf.call(&mut store, ())?;
let status = set_color_abi.call(&mut store, (addr, r, g, b))?;
if status == 0 {
    let data = memory.data(&store);
    let mut words = [0u32; 3];
    for (i, w) in words.iter_mut().enumerate() {
        let at = addr as usize + 4 * i;
        *w = u32::from_le_bytes(data[at..at + 4].try_into().unwrap());
    }
    // words 即结果，按示例约定解码
}
```

反向（guest 调 host）对称：host 侧写业务函数与 `abi! { host … }` 声明、交给运行时注册为导入实现；
`ctx` 放第一位供给运行时句柄；guest 侧手写 `unsafe extern "C"` 导入声明并调用。

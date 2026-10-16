# 缓冲区协议示例

[文档首页](../README.zh-CN.md) · [使用指南](guide.md)

本页演示如何传递超过一个槽的数据。**以下约定全部由使用方定义，适配器不生成或检查它们。**
构建设置见[导出 wasm 函数](guide.md#导出-wasm-函数)。

## 示例约定

wasm 函数形状为 `(buf_addr, r, g, b) -> status`，四个参数和返回值均为 32 位槽：

- 模块保留一块容量为 3 个 `u32` 的缓冲，宿主通过 `__abi_ret_buf` 取得它的起始地址。
- `buf_addr` 必须恰好等于该起始地址，其他地址一律拒绝。
- 状态 `0` 表示成功，结果按小端字节序写入；其他状态表示失败，原缓冲不变，宿主不读取结果。
- 实例为单线程、非共享内存。宿主只在导出调用之间访问缓冲，不并发访问或重入调用。

这里的地址只用于确认缓冲身份。写入时始终使用模块从自身静态数组取得的有效指针，
不把外部传入的任意整数当作可写指针。

## wasm 模块代码

```rust
// The application defines Rgb and its slot representation.
pub struct Rgb { r: u32, g: u32, b: u32 }

impl abi_adapter::FromSlots<3> for Rgb {
    fn from_slots([r, g, b]: [u32; 3]) -> Self {
        Self { r, g, b }
    }
}

static mut RET_BUF: [u32; 3] = [0; 3];

/// Return the reserved buffer address to the host.
#[unsafe(export_name = "__abi_ret_buf")]
pub extern "C" fn abi_ret_buf() -> u32 {
    (&raw mut RET_BUF).expose_provenance() as u32
}

/// Validate the address and write the result in application code.
pub fn set_color(buf_addr: u32, color: Rgb) -> u32 {
    let out = &raw mut RET_BUF;
    if buf_addr == 0 || buf_addr as usize != out.addr() {
        return 1;
    }
    let words = [color.r.to_le(), color.g.to_le(), color.b.to_le()];
    // SAFETY: out points to the writable static array RET_BUF. It is valid,
    // aligned, and holds exactly three slots. In this single-threaded instance
    // with unshared memory, no host access, reentrant call, or live reference
    // overlaps this write.
    unsafe { out.write(words); }
    0
}

abi_adapter::abi! {
    guest("set_color.v1") pub fn set_color_abi([buf_addr], [r, g, b])
        -> 1 => crate::set_color;
}
```

## 宿主读取结果

下面以 JavaScript 宿主为例，`instance` 是已加载的 wasm 实例：

```javascript
const api = instance.exports;
const address = api.__abi_ret_buf() >>> 0;
const status = api["set_color.v1"](address, 10, 20, 30);
if (status === 0) {
  // Refresh the view after the call, which may have grown memory.
  const view = new DataView(api.memory.buffer);
  const words = [0, 1, 2].map((i) => view.getUint32(address + 4 * i, true));
  // words is [10, 20, 30]; true selects little-endian byte order.
}
```

`>>> 0` 将 JavaScript 收到的有符号 `i32` 结果解释为无符号地址。
内存增长不会改变模块静态缓冲的线性内存偏移，但可能使宿主旧有的内存视图失效。
本例由 [`tests/doc-buffer.cjs`](../tests/doc-buffer.cjs) 直接提取并编译为 wasm，
检查正确写入，以及零地址、未对齐地址、缓冲内部或末尾、越界和其他分配的地址均被拒绝。

## 设计其他缓冲协议时

若采用不同的内存协议，使用方需要保证缓冲有效、可写、容量足够、调用期间存活，
且没有冲突的借用或并发写入。仅检查地址非零或位于 wasm 线性内存范围内，
不能证明这些条件。宿主局部变量地址也不能作为模块内存地址使用。

整数地址不包含所有指针安全信息。若确实需要把指针传为整数后再还原，
必须从真实分配的可写指针使用 `expose_provenance` 导出地址，再使用
`with_exposed_provenance_mut` 恢复其来源信息（Rust 称为 provenance）；
单独使用 `addr()` 不够，恢复来源也不会自动验证容量或访问权限。
持有原始有效指针时，可像上例一样只验证地址身份，写入仍使用该指针。
静态缓冲的写指针应从 `&raw mut` 获取，不能用 `&raw const` 代替。

`write_unaligned` 只放宽对齐要求，仍要求有效的可写内存。
构造 `&mut [u32]` 或直接写入 `u32` 指针则必须满足 `u32` 对齐要求。
用一个 `u32` 传地址的约定仅适用于 32 位指针，不能直接用于 64 位 wasm 内存。
内存可能增长时，宿主不要跨调用缓存裸指针或旧内存视图。

更详细的 Rust 安全要求见标准库文档中的
[原始指针与访问权限](https://doc.rust-lang.org/stable/core/ptr/macro.addr_of.html)和
[可变切片构造](https://doc.rust-lang.org/stable/core/slice/fn.from_raw_parts_mut.html)。

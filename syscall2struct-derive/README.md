# syscall2struct-derive

`syscall2struct-derive` 提供了两个 derive macro，能够自动生成 `syscall2struct_helpers::MakeSyscall` 和 `syscall2struct_helpers::MakeSyscallMut` 的实现。

## 示例

使用 syzlang 描述（假设系统调用号为 123）：

```
resource fd[int32]: -1

foo(a int32, in_buf ptr[in, filename], handle resource)
bar(b int64, out_buf ptr[out, string]) fd
```

使用 `syscall2struct-derive` 描述：

```rust
use syscall2struct_derive::*;  // Derive macro
use syscall2struct_helpers::*; // Helper structs and traits

use heapless::Vec;      // For buffer type
use serde::Deserialize; // Deserialize testcase in bytes
use syscalls::raw::*;   // Make syscalls
use uuid::Uuid;         // For syscall result type

// This is a fake syscall
#[derive(Deserialize, MakeSyscall)]
#[sysno(123)]
pub struct Foo {
    pub a: u64,
    #[in_ptr]
    pub in_buf: Pointer<Vec<u8, 4096>>,
    pub handle: Result,
}

#[derive(Deserialize, MakeSyscallMut)]
#[sysno(456)]
pub struct Bar {
    pub b: u64,
    #[out_ptr]
    pub out_buf: Pointer<Vec<u8, 4096>>,
    #[ret_val]
    pub id: Uuid,
}
```

### 基本结构

- `use` 语句：使用 `syscall2struct-derive` 所必需的依赖包括：

    - `syscall2struct-derive::*`

    - `syscall2struct_helpers::*`

    - `syscalls::raw::*`

    其余如 `heapless::Vec`、`uuid::Uuid` 等视情况添加。

- `#[derive(Deserialize, MakeSyscall)]`：为结构体添加 `Deserialize` 和 `MakeSyscall` 的实现，用于反序列化测例和发出系统调用。

- `#[sysno(123)]`：指定系统调用号。

- `struct Foo`：为每一个系统调用定义一个结构体，建议将对应的系统调用名称作为结构体名称。

- `pub a: u64` 等：为每一个系统调用的参数定义一个字段，其类型应与 syzlang 描述中的类型相匹配，具体对应关系见下。

- `syscall2struct-derive` 对所作用的结构体及其字段的可见性**没有**要求，`pub` 仅为示例。

### 类型描述

- `int8`、`int16`、`int32`、`int64`、`flags` 及任何底层为整型的类型：均使用 `u64`，以保证能够容纳所有整型数据。由于系统调用参数的类型是 `usize`，在 32 位机器上使用 `int64` 数据可能导致截断。

- `ptr[dir, type]`：使用 `Pointer<T>`，其中 `T` 为 `type` 对应的类型。若 `dir` 为 `in`，用 `#[in_ptr]` 标记该字段；若 `dir` 为 `out`，用 `#[out_ptr]` 标记该字段。

- `string`、`filename`、`buffer`：使用 `Vec<u8, N>`，其中 `N` 为最大长度。

- `resource`：使用 `SyscallResult`。

- 系统调用返回值 ID（如示例中的 `id` 字段）：使用 `Uuid`，并用 `#[ret_val]` 标记该字段。系统调用返回值 ID 由 fuzzer 分配并传递给 harness，解析测例时会得到 ID 并保存在结构体中。发出系统调用时不会使用 ID，只有发出系统调用（例如通过 `MakeSyscall:call` 或 `MakeSyscallMut::call`）之后才能得到系统调用返回值的具体值。将 ID 和返回值存入 `ResultContainer` 即可供后续系统调用使用。

`array`、`struct`、`union` 等暂不支持。
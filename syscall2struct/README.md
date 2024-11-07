# syscall2struct

`syscall2struct` 是一个配合 [flicker](https://github.com/nine-point-eight-p/flicker) 使用的辅助编写 harness 的工具。它利用 `syscall2struct-helpers` 和 `syscall2struct-derive`，自动将 syzlang 描述的系统调用转换为 Rust 结构体，同时为其实现 `serde::Deserialize` 和 `syscall2struct_helpers::MakeSyscall`/`syscall2struct_helpers::MakeSyscallMut` 等 trait，从而支持解析测例、发出系统调用等功能，减少编写 harness 的任务量。

`syscall2struct` 将生成一个新的 Rust 项目，其目录结构如下：

```
my-harness/
├── Cargo.lock
├── Cargo.toml
└── src
    ├── main.rs
    └── syscall.rs
```

其中 `src/main.rs` 包含了待完成的 harness 主体部分，`src/syscall.rs` 包含了所有系统调用的结构体定义。关于 harness 的具体实现，可参考[测试 Alien 所用的 harness](https://github.com/nine-point-eight-p/Alien/tree/harness/user/apps/harness)。

## 如何使用

```
$ cargo run --release -- --help
Generate Rust structs from syscall definitions written in Syzlang

Usage: syscall2struct [OPTIONS]

Options:
      --desc <DESC>        Path to the description file [default: desc/test.txt]
      --const <CONST>      Path to the constants file [default: desc/test.txt.const]
      --package <PACKAGE>  Path to the generated project, must not exist yet [default: ../my-harness]
  -h, --help               Print help
  -V, --version            Print version
```

- `--desc` 指定 syzlang 描述文件的路径，默认为 `desc/test.txt`。

- `--const` 指定 syzlang 常量文件的路径，默认为 `desc/test.txt.const`。

- `--package` 指定生成的项目路径，默认为 `../my-harness`。注意该路径必须不存在已有项目，否则无法创建新的项目。

在仓库**根目录**下执行 `cargo run --release` 即可生成示例 harness 项目。
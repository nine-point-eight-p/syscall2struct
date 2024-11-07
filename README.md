# syscall2struct

`syscall2struct` 是一个配合 [flicker](https://github.com/nine-point-eight-p/flicker) 使用的工具，目标是将 syzlang 编写的系统描述文件自动转化为与 flicker 适配的 harness，简化测试流程。

`syscall2struct` 由三个 crate 组成：

- `syscall2struct`：工具本体，能够生成一个 Rust 项目作为 harness 的基础。

- `syscall2struct-derive`：定义了一些 derive macro，自动生成系统调用结构体的部分实现代码。

- `syscall2struct-helpers`：定义了一些辅助性的 trait 与结构体。

其中，只有 `syscall2struct` 是可执行程序，它在生成的 Rust 项目中使用 `syscall2struct-derive` 与 `syscall2struct-helpers` 提供的功能。用户也可以直接使用后两者自行编写 harness。
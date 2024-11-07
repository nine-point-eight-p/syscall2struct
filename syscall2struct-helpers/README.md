# syscall2struct-helpers

`syscall2struct-helpers` 为 `syscall2struct` 提供了一些辅助性的 trait 和结构体，主要包括两个部分：

- 为了更方便地发出系统调用，提供了两个 trait：

    - `MakeSyscall`：描述无输出到参数（即参数不可变）的系统调用。

    - `MakeSyscallMut`：描述有输出到参数（即参数可变）的系统调用。

- 为了更方便地解析 fuzzer 传递的测例，提供了

    - `Pointer`：表示指针类型数据，包含完整数据或指定地址值。可能包含一个完整数据，取其地址作为系统调用的参数；也可能包含一个地址值，直接作为系统调用的参数。

    - `SyscallResult`：表示系统调用的结果。可能包含一个 UUID，表示所使用的系统调用结果的索引；也可能包含一个具体值。

    - `ResultContainer`：系统调用结果的键值容器，使用 fuzzer 传递的 UUID 作为索引。
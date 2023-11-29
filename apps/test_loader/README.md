[toc]

----

# 动态链接 c app 到自定义 libc 中

> 只实现了装载时重定位

## 运行流程

> 需要开启 multitask, paging feature

编译运行
- 在应用目录下执行 `python3 build.py`
  - 编译每个 c 文件, 不加任何参数, 生成动态链接的 ELF 文件
  - 为每个 elf 文件加上一个 header, 作用类似于 inode, 指示文件的大小
  - 将文件合并为一个 32M 的大文件, 复制到 `payload/` 里
- 在 arceos 目录下 `make run`

系统进行初始化后运行 main
- 读取 plash 里的各个 elf 文件
- 对 elf 文件重定位
- 为每个 app 生成一个任务, 加入任务队列
- 使用 join 函数等待所有任务完成后, 自己退出

每个 app 运行的流程
- 系统调度, app 上处理机
- 进入 task_entry 函数, 其会跳转到 app 的第一个函数 _start
- 进入 _start 函数, 不久后跳转到 __libc_start_main
- __libc_start_main 的位置已被动态重定位到自定义 libc 中
- 自定义的 __libc_start_main 直接跳转到 c app 的 main
- main 运行结束, 回到 __libc_start_main, 再回到 task_entry
- task_entry 执行 exit, 一个 app 执行结束, 发生调度

## main 的功能

config: 
- c app 起始虚拟地址
- plash 的物理地址

file:
- 从 plash 读取文件
- 返回各个文件的内容
- 文件的前 24 字节是自定义的元信息, 用于指示文件的大小

dl
- 解析 elf 文件
- 重定位

mylibc
- 自定义 c 库
- 提供了 __libc_main_start, puts, sleep(yield) 函数


## 修改内核

axhal/cpu.rs
- CURRENT_TASK_PTR: 
  - 值是当前任务 TCB 的地址
  - 原本, 该变量被永久的保存到 gp 寄存器里, 但是发现 musl 编译的程序会修改 gp 寄存器, 导致内核出现问题
  - 于是, 修改该变量, 仅作为一个全局变量, 保存在内存里

axtask/api.rs
- 新增函数 `new_from_data()`
  - 参数: entry, 表示 app 入口虚拟地址
  - 参数: datas, 类型是 `Vec<(usize, Vec<u8>)>`, 表示 app 所有 load 段的数据, 以及他们的起始虚拟地址
  - 行为: 新建一个 TCB, 加入准备队列, 并保存到`ALL_TASKS`里
- 新增全局变量 `ALL_TASKS`: 
  - 保存 `new_from_data` 新增的任务的 TCB
- 新增函数 `join_all()`
  - 行为: 依次 join `ALL_TASKS` 里的 TCB

axtask/run_queue.rs
- 修改函数 `init()`
  - 增加一行: 在 main 初始化完成之后, 将页表切换为 main 的页表
- 修改函数 `switch_to()`
  - 增加一行: 在切换前, 也把页表寄存器给切换成下一个任务的页表

axtask/task.rs
- 修改结构体 `TaskInner`
  - 增加一个项, `pagetable`, 保存当前任务对应的页表
- 增加函数 `set_root_pagetable()`, 将页表寄存器切换成自己的页表
- 增加函数 `pagetable_ptr_mut()`, 返回自己的页表的指针
- 修改 `new_common()`, 现在创建 TCB 的时候, 会新建一个页表, 并且映射内核的空间, 再保存到 TCB 里
- 增加函数 `new_from_data()`
  - 主要参数有 app 的数据, 入口地址
  - 先用 `new_common()` 生成一个 TCB
  - 根据 app 数据, 申请一定的物理页, 将数据写入物理页
  - 新建一个页表, 根据 app 数据, 映射其虚拟地址到申请的物理页
  - 将 TCB 的页表改成刚刚新建的页表
  - 修改任务上下文, 将入口地址放入上下文的 s11 寄存器里
  - 返回 tcb
- 修改函数 `task_entry()`
  - 判断是否要从 `s11` 寄存器读取入口地址来跳转


api/arceos_api/imp/ 新增文件 multi_app.rs
- 提供函数用于页表映射和解除, 只用于映射 plash 
- 提供函数调用 axtask/api.rs 里新增的函数


[toc]

----

# 使用装入时重定位运行 Linux 原始应用

## 运行方法

编译`C`源码并运行
- 运行`make run DL=y`
- 这将会编译`linux_apps/c`中的两个`C`测例
- 并用`arceos`运行

运行在`riscv Linux`上编译好的`C`程序
```sh
cd apps/linux_apps 
python3 test.py 
cd - 
make run 
```

修改调度算法
- 默认使用`fifo`调度
- 可以用`FEATURES=sched_rr`或`FEATURES=sched_cfs`改成别的算法


## 具体细节

编译并打包`C`程序
- 生成动态链接的`ELF`文件
- 在每个文件的二进制数据的开头加上一个`header`, 用于指示文件的大小
- 将所有文件合并为的`apps.bin`, 复制到`payload/`

`linux_apps main()`
- 从`plash`读取`ELF`文件, 并重定位
- 为每个`app`生成一个任务, 加入任务队列
- 使用`join()`, 等待所有任务完成后, 退出

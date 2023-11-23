## 问题

在练习 6 中, 修改示例代码的 `init_app_page_table()`的最后一行, 将页表映射偏移一页
```rust
unsafe fn init_app_page_table() {
    // 0x8000_0000..0xc000_0000, VRWX_GAD, 1G block
    APP_PT_SV39[2] = (0x80000 << 10) | 0xef;
    // 0xffff_ffc0_8000_0000..0xffff_ffc0_c000_0000, VRWX_GAD, 1G block
    APP_PT_SV39[0x102] = (0x80000 << 10) | 0xef;

    // 0x0000_0000..0x4000_0000, VRWX_GAD, 1G block
    APP_PT_SV39[0] = (0x00000 << 10) | 0xef;

    // For App aspace!
    // 0x4000_0000..0x8000_0000, VRWX_GAD, 1G block
    // APP_PT_SV39[1] = (0x80000 << 10) | 0xef;
    APP_PT_SV39[1] = (0x80001 << 10) | 0xef;
}
```

随后在 `main()` 里直接进行访问
```rust
fn main() {
    unsafe {
        init_app_page_table();
        switch_app_aspace();
        println!("0x8010_1000 = {}", *((0x8010_1000) as *const u8));
        println!("0x4010_0000 = {}", *(0x4010_0000 as *const u8));
    }
}
```

第一个`println`输出 0, 第二个会发生错误
```
Unhandled trap Exception(LoadPageFault) @ 0xffffffc0802020ec:
TrapFrame {
    regs: GeneralRegisters {
        ra: 0xffffffc080202034,
        sp: 0xffffffc080248c60,
        gp: 0x0,
        tp: 0x0,
        t0: 0xffffffc080248eb8,
        t1: 0xffffffc0802010f0,
        t2: 0x1000,
        s0: 0xffffffc080248da0,
        s1: 0xffffffc080203048,
        a0: 0x40100000,
        a1: 0xffffffc080248c68,
        a2: 0xffffffc0802020ec,
        a3: 0xffffffc080201676,
        a4: 0x0,
        a5: 0xffffffc080201676,
        a6: 0x110000,
        a7: 0x1,
        s2: 0x1,
        s3: 0xffffffc080248d60,
        s4: 0xffffffc080248db0,
        s5: 0xffffffc0802067ff,
        s6: 0xef,
        s7: 0x200004ef,
        s8: 0x8000000000080206,
        s9: 0x200000ef,
        s10: 0x40100000,
        s11: 0x0,
        t3: 0x8001132e,
        t4: 0xffffffc080203de0,
        t5: 0x27,
        t6: 0x0,
    },
    sepc: 0xffffffc0802020ec,
    sstatus: 0x8000000000006100,
}
```


## 结论

> Any level of PTE may be a leaf PTE, so in addition to 4 KiB pages, Sv39 supports 2 MiB megapages and 1 GiB gigapages, each of which must be virtually and physically aligned to a boundary equal to its size. A page-fault exception is raised if the physical address is insufficiently aligned.
https://github.com/riscv/riscv-isa-manual/blob/main/src/supervisor.adoc

如果不对齐会在地址翻译的这里发生异常
> 6. If i>0 and pte.ppn[i-1:0] ≠ 0, this is a misaligned superpage; stop and raise a page-fault exception corresponding to the original access type.
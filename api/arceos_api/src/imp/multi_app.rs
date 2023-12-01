/// plain map for len MiB
pub fn pagetable_map_mib(paddr: usize, len: usize) {
    use axhal::paging::MappingFlags;
    use axhal::paging::PageSize;
    assert_eq!(len % 2, 0);
    for i in 0..len / 2 {
        let addr = paddr + 2 * i * (1024 * 1024);
        unsafe {
            let pt = axtask::current().pagetable_ptr_mut();
            (*pt)
                .map(
                    addr.into(),
                    addr.into(),
                    PageSize::Size2M,
                    MappingFlags::READ | MappingFlags::WRITE,
                )
                .unwrap();
        }
    }
}

/// unmap pflash from paddr start start for len MiB
pub fn pagetable_unmap_mib(paddr: usize, len: usize) {
    assert_eq!(len % 2, 0);
    for i in 0..len / 2 {
        let addr = paddr + 2 * i * (1024 * 1024);
        unsafe {
            let pt = axtask::current().pagetable_ptr_mut();
            (*pt).unmap(addr.into()).unwrap();
        }
    }
}

/// `data` shell be a Vec<(start_va: usize, Vec<u8>)> to guide where to put data
pub fn create_task_from_data(datas: alloc::vec::Vec<(usize, alloc::vec::Vec<u8>)>, entry: usize) {
    axtask::new_from_data(datas, entry);
}

/// wait all tasks you just generate
pub fn join_all() {
    axtask::join_all();
}

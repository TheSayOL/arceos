pub fn map_data(data: &[u8], va: usize, len: usize) {
    let mut va = va as *mut u8;
    unsafe {
        let va = core::slice::from_raw_parts_mut(va, len);
        va.copy_from_slice(data);
    }
}

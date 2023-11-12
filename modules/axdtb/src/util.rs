extern crate alloc;

use alloc::vec::Vec;
use dtb::Reader;

pub struct DeviceTree<'a> {
    inner: Reader<'a>,
}

impl<'a> DeviceTree<'a> {
    pub fn new(ptr: usize) -> Self {
        let inner: Reader<'_> = unsafe { Reader::read_from_address(ptr).unwrap() };
        Self { inner }
    }

    pub fn memory_addr_size(&self) -> (usize, usize) {
        let items = self.inner.struct_items();
        let mut addr = 0;
        let mut size = 0;
        let mut found = false;
        for item in items {
            if item.is_begin_node() && item.node_name().unwrap() == "memory" {
                found = true;
            }
            if item.is_property() && found && item.name().unwrap() == "reg" {
                let len = item.value().unwrap().len() / 4;
                for i in 0..len {
                    let mut slice = [0u8; 4];
                    slice.copy_from_slice(&item.value().unwrap()[i * 4..(i + 1) * 4]);
                    if i == 1 {
                        addr = u32::from_be_bytes(slice);
                    }
                    if i == 3 {
                        size = u32::from_be_bytes(slice);
                    }
                }
                break;
            }
        }
        (addr as usize, size as usize)
    }

    pub fn mmio_regions(&self) -> Vec<(usize, usize)> {
        let mut v = Vec::new();
        let items = self.inner.struct_items();
        let mut found = false;
        for item in items {
            if item.is_begin_node() {
                if item.node_name().unwrap() == "virtio_mmio" {
                    found = true;
                } else {
                    found = false;
                }
            }
            if item.is_property() && found && item.name().unwrap() == "reg" {
                let mut addr = 0;
                let mut size = 0;
                let len = item.value().unwrap().len() / 4;
                for i in 0..len {
                    let mut slice = [0u8; 4];
                    slice.copy_from_slice(&item.value().unwrap()[i * 4..(i + 1) * 4]);
                    if i == 1 {
                        addr = u32::from_be_bytes(slice);
                    }
                    if i == 3 {
                        size = u32::from_be_bytes(slice);
                    }
                }
                v.push((addr as usize, size as usize));
            }
        }
        v
    }
}

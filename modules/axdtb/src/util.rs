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
            } else if item.is_property() && found && item.name().unwrap() == "reg" {
                let mut slice = [0u8; 4];
                // memory addr
                slice.copy_from_slice(&item.value().unwrap()[4..8]);
                addr = u32::from_be_bytes(slice);
                // size
                slice.copy_from_slice(&item.value().unwrap()[12..16]);
                size = u32::from_be_bytes(slice);
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
                found = match item.node_name().unwrap() {
                    "virtio_mmio" => true,
                    _ => false,
                };
            } else if item.is_property() && found && item.name().unwrap() == "reg" {
                let mut slice = [0u8; 4];
                // addr
                slice.copy_from_slice(&item.value().unwrap()[4..8]);
                let addr = u32::from_be_bytes(slice);
                // size
                slice.copy_from_slice(&item.value().unwrap()[12..16]);
                let size = u32::from_be_bytes(slice);
                v.push((addr as usize, size as usize));
            }
        }
        v
    }
}

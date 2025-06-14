#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

#[macro_use]
extern crate log;
extern crate alloc;

use alloc::boxed::Box;
use alloc::vec;
use allocator::UEFIFrameAllocator;
use elf::{load_elf, map_physical_memory, map_pages};
use fs::{load_file, open_file};
use uefi::mem::memory_map::MemoryMap;
use uefi::proto::debug;
use uefi::{Status, entry};
use x86_64::registers::control::*;
use xmas_elf::ElfFile;
use ysos_boot::config::Config;
use ysos_boot::*;

mod config;

const CONFIG_PATH: &str = "\\EFI\\BOOT\\boot.conf";

#[entry]
fn efi_main() -> Status {
    uefi::helpers::init().expect("Failed to initialize utilities");

    log::set_max_level(log::LevelFilter::Trace);
    info!("Running UEFI bootloader...");

    // 1. Load config
    let boot_config_bytes: &[u8] = include_bytes!(
        "/Users/warblerforest/Projects/YatOS/Lab/ForestWarbler-YatSenOS-V2/pkg/kernel/config/boot.conf"
    );
    let config = Config::parse(boot_config_bytes);

    info!("Config: {:#x?}", config);

    // 2. Load ELF files
    let mut elf_file = open_file(config.kernel_path);
    let elf_buf = load_file(&mut elf_file);
    let elf = ElfFile::new(elf_buf).expect("Failed to load ELF file");

    unsafe {
        set_entry(elf.header.pt2.entry_point() as usize);
    }

    // 3. Load MemoryMap
    let mmap = uefi::boot::memory_map(MemoryType::LOADER_DATA).expect("Failed to get memory map");

    let max_phys_addr = mmap
        .entries()
        .map(|m| m.phys_start + m.page_count * 0x1000)
        .max()
        .unwrap()
        .max(0x1_0000_0000); // include IOAPIC MMIO area

    // 4. Map ELF segments, kernel stack and physical memory to virtual memory
    let mut page_table = current_page_table();

    // FIXME: root page table is readonly, disable write protect (Cr0)
    unsafe {
        Cr0::update(|mut flags| {
            flags.remove(Cr0Flags::WRITE_PROTECT);
        });
    }

    // FIXME: map physical memory to specific virtual address offset
    let mut frame_allocator = UEFIFrameAllocator;

    map_physical_memory(
        config.physical_memory_offset,
        max_phys_addr,
        &mut page_table,
        &mut frame_allocator,
    );

    // FIXME: load and map the kernel elf file
    let kernel_pages = load_elf(
        &elf,
        config.physical_memory_offset,
        &mut page_table,
        &mut frame_allocator,
        false,
    )
    .expect("Failed to load ELF file");

    let (stack_start, stack_size) = if config.kernel_stack_auto_grow > 0 {
        let init_size = config.kernel_stack_auto_grow;
        let bottom_offset = (config.kernel_stack_size - init_size) * 0x1000;
        let init_bottom = config.kernel_stack_address + bottom_offset;
        (init_bottom, init_size)
    } else {
        (config.kernel_stack_address, config.kernel_stack_size)
    };

    debug!(
        "Kernel stack start: {:#x}, size: {:#x}",
        stack_start, stack_size
    );

    // FIXME: map kernel stack
    map_pages(
        stack_start,
        stack_size,
        &mut page_table,
        &mut frame_allocator,
        false,
        false,
    )
    .expect("Failed to map kernel stack");

    // FIXME: recover write protect (Cr0)
    unsafe {
        Cr0::update(|mut flags| {
            flags.insert(Cr0Flags::WRITE_PROTECT);
        });
    }

    free_elf(elf);

    let apps = if config.load_apps {
        info!("Loading apps...");
        Some(load_apps())
    } else {
        info!("Skip loading apps");
        None
    };

    // 5. Pass system table to kernel
    let ptr = uefi::table::system_table_raw().expect("Failed to get system table");
    let system_table = ptr.cast::<core::ffi::c_void>();

    // 6. Exit boot and jump to ELF entry
    info!("Exiting boot services...");

    let mmap = unsafe { uefi::boot::exit_boot_services(MemoryType::LOADER_DATA) };
    let kernel_pages: KernelPages = kernel_pages.iter().map(|page_range| *page_range).collect();
    // NOTE: alloc & log are no longer available

    // construct BootInfo
    let bootinfo = BootInfo {
        memory_map: mmap.entries().copied().collect(),
        physical_memory_offset: config.physical_memory_offset,
        system_table,
        log_level: config.log_level,
        loaded_apps: apps,
        kernel_pages,
    };

    // align stack to 8 bytes
    let stacktop = config.kernel_stack_address + config.kernel_stack_size * 0x1000 - 8;

    jump_to_entry(&bootinfo, stacktop);
}

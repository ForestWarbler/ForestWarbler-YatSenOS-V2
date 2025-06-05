#![no_std]
#![no_main]

use log::*;
use storage::Block;
use storage::FsError;
use storage::PartitionTable;
use storage::mbr::MbrTable;
use uefi::proto::debug;
use uefi::proto::media::partition;
use ysos::*;
use ysos_kernel as ysos;
use ysos_kernel::drivers::ata::AtaDrive;
use ysos_kernel::drivers::filesystem;

extern crate alloc;

boot::entry_point!(kernel_main);

pub fn kernel_main(boot_info: &'static boot::BootInfo) -> ! {
    ysos::init(boot_info);
    drive_init();
    filesystem::init();
    ysos::wait(spawn_init());
    ysos::shutdown();
}

pub fn spawn_init() -> proc::ProcessId {
    // NOTE: you may want to clear the screen before starting the shell
    // print_serial!("\x1b[1;1H\x1b[2J");

    proc::list_app();
    proc::spawn("fwsh").unwrap()
}

pub fn drive_init() {
    let drive = AtaDrive::open(0, 0).expect("Failed to open ATA drive 0:0");
    let mbr: MbrTable<_, Block<512>> = MbrTable::parse(drive).expect("Failed to parse MBR");
    let partitions = mbr.partitions().expect("Failed to get partitions");
    if partitions.is_empty() {
        error!("No active partitions found");
        ysos::shutdown();
    }
    for part in &partitions {
        info!("Partition: {:#?}", part);
    }
}

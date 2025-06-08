use super::ata::*;
use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use chrono::DateTime;
use storage::fat16::Fat16;
use storage::mbr::*;
use storage::*;

pub static ROOTFS: spin::Once<Mount> = spin::Once::new();

pub fn get_rootfs() -> &'static Mount {
    ROOTFS.get().unwrap()
}

pub fn init() {
    info!("Opening disk device...");

    let drive = AtaDrive::open(0, 0).expect("Failed to open disk device");

    // only get the first partition
    let part = MbrTable::parse(drive)
        .expect("Failed to parse MBR")
        .partitions()
        .expect("Failed to get partitions")
        .remove(0);

    info!("Mounting filesystem...");

    ROOTFS.call_once(|| Mount::new(Box::new(Fat16::new(part)), "/".into()));

    trace!("Root filesystem: {:#?}", ROOTFS.get().unwrap());

    info!("Initialized Filesystem.");
}

pub fn ls(root_path: &str) {
    // info!("Listing files in '{}'", root_path);
    let iter = match get_rootfs().read_dir(root_path) {
        Ok(iter) => iter,
        Err(err) => {
            warn!("{:?}", err);
            return;
        }
    };

    // FIXME: format and print the file metadata
    //      - use `for meta in iter` to iterate over the entries
    //      - use `crate::humanized_size_short` for file size
    //      - add '/' to the end of directory names
    //      - format the date as you like
    //      - do not forget to print the table header
    let mut entries: Vec<Metadata> = iter.collect();

    entries.sort_by(|a, b| match (a.is_dir(), b.is_dir()) {
        (true, false) => core::cmp::Ordering::Less,
        (false, true) => core::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    let name_w = entries
        .iter()
        .map(|m| m.name.len() + if m.is_dir() { 1 } else { 0 })
        .max()
        .unwrap_or(4); // "Name"
    let size_w = entries
        .iter()
        .map(|m| {
            if m.is_dir() {
                1
            } else {
                let (num, unit) = crate::humanized_size_short(m.len as u64);
                format!("{:.1}{}", num, unit).len()
            }
        })
        .max()
        .unwrap_or(4); // "Size"

    println!(
        "{:<name_w$}  {:>size_w$}  {:<}",
        "Name",
        "Size",
        "Modified",
        name_w = name_w,
        size_w = size_w
    );
    println!(
        "{:-<name_w$}  {:-<size_w$}  {:-<}",
        "",
        "",
        "",
        name_w = name_w,
        size_w = size_w
    );

    for meta in entries {
        let mut name = meta.name.clone();
        if meta.is_dir() {
            name.push('/');
        }

        let size_str = if meta.is_dir() {
            "-".to_string()
        } else {
            let (num, unit) = crate::humanized_size_short(meta.len as u64);
            format!("{:.1}{}", num, unit)
        };

        let time_str = if let Some(t) = meta.modified {
            let dt: DateTime<chrono::Utc> = t.into();
            format!("{}", dt.format("%Y-%m-%d %H:%M"))
        } else {
            "-".into()
        };

        println!(
            "{:<name_w$}  {:>size_w$}  {:<}",
            name,
            size_str,
            time_str,
            name_w = name_w,
            size_w = size_w,
        );
    }
}

pub fn check_dir_exists(path: &str) -> bool {
    match get_rootfs().metadata(path) {
        Ok(meta) => meta.is_dir(),
        Err(_) => false,
    }
}

pub fn cat(path: &str) -> Option<String> {
    let mut file = get_rootfs().open_file(path).ok()?;

    let mut buf = Vec::<u8>::new();
    if let Ok(meta) = get_rootfs().metadata(path) {
        buf.reserve(meta.len);
    }

    let mut tmp = [0u8; 512];
    loop {
        let n = file.read(&mut tmp).ok()?;
        if n == 0 {
            break; // EOF
        }
        buf.extend_from_slice(&tmp[..n]);
    }

    let s = match core::str::from_utf8(&buf) {
        Ok(text) => text.to_string(),
        Err(_) => buf.iter().map(|b| format!("{:02X} ", b)).collect(),
    };

    println!("{}", s);

    Some(s)
}

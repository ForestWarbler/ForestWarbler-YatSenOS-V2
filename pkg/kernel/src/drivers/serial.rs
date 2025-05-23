use super::uart16550::SerialPort;

const SERIAL_IO_PORT: u16 = 0x3F8; // COM1

once_mutex!(pub SERIAL: SerialPort);

pub fn init() {
    init_SERIAL(SerialPort::new(SERIAL_IO_PORT));
    get_serial_for_sure().init();

    print!("\x1B[2J\x1B[H");

    println!("{}", crate::get_ascii_header());
    println!("[+] Serial Initialized.");
}

guard_access_fn!(pub get_serial(SERIAL: SerialPort));

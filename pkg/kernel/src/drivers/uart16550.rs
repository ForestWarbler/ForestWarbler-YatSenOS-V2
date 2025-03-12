use core::fmt;
use x86_64::instructions::port::Port;
use bitflags::bitflags;

/// A port-mapped UART 16550 serial interface.
pub struct SerialPort;

impl SerialPort {
    pub const fn new(port: u16) -> Self {
        Self
    }

    /// Initializes the serial port.
    pub fn init(&self) {
        // FIXME: Initialize the serial port
        let mut base= 0x3F8;
        let mut buffer: Port<u8> = Port::new(base);
        let mut interrupt_enable: Port<u8> = Port::new(base + 1);
        let mut fifo_control: Port<u8> = Port::new(base + 2);
        let mut line_control: Port<u8> = Port::new(base + 3);
        let mut modem_control: Port<u8> = Port::new(base + 4);

        bitflags! {
            pub struct LineControlFlags: u8 {
                const DATA_BITS_5 = 0x00;
                const DATA_BITS_6 = 0x01;
                const DATA_BITS_7 = 0x02;
                const DATA_BITS_8 = 0x03;
                const STOP_BITS_1 = 0x00;
                const STOP_BITS_2 = 0x04;
                const PARITY_NONE = 0x00;
                const PARITY_ODD = 0x08;
                const PARITY_EVEN = 0x18;
                const PARITY_MARK = 0x28;
                const PARITY_SPACE = 0x38;
                const DLAB = 0x80;
            }
        }

        unsafe {
            interrupt_enable.write(0x00u8); // Disable all interrupts
            line_control.write(LineControlFlags::DLAB.bits()); // Enable DLAB (set baud rate divisor)
            buffer.write(0x03u8); // Set divisor to 3 (lo byte) 38400 baud
            interrupt_enable.write(0x00u8); //                  (hi byte)
            line_control.write(LineControlFlags::DATA_BITS_8.bits()); // 8 bits, no parity, one stop bit
            fifo_control.write(0xC7u8); // Enable FIFO, clear them, with 14-byte threshold
            modem_control.write(0x0Bu8); // IRQs enabled, RTS/DSR set
            modem_control.write(0x1Eu8); // Set in loopback mode, test the serial chip
            buffer.write(0xAEu8); // Test serial chip (send byte 0xAE and check if serial returns same byte)
        
            if buffer.read() != 0xAEu8 {
                panic!("Serial port initialization failed");
            }

            modem_control.write(0x0Fu8); // Normal operation
        }
    }

    /// Sends a byte on the serial port.
    pub fn send(&mut self, data: u8) {
        // FIXME: Send a byte on the serial port
        let base = 0x3F8;
        let mut buffer: Port<u8> = Port::new(base);
        let mut line_status: Port<u8> = Port::new(base + 5);

        unsafe {
            while (line_status.read() & 0x20u8) == 0 {
                // Wait for the transmit buffer to be empty
            }
    
            buffer.write(data);
        }
    }

    /// Receives a byte on the serial port no wait.
    pub fn receive(&mut self) -> Option<u8> {
        // FIXME: Receive a byte on the serial port no wait
        let base = 0x3F8;
        let mut buffer: Port<u8> = Port::new(base);
        let mut line_status: Port<u8> = Port::new(base + 5);
        
        unsafe {
            while (line_status.read() & 0x01u8) == 0 {
                // Wait for the receive buffer to be full
            }
    
            Some(buffer.read())
        }
    }
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.send(byte);
        }
        Ok(())
    }
}

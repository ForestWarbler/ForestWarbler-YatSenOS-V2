use core::fmt;
use x86_64::instructions::port::Port;

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

        unsafe {
            interrupt_enable.write(0x00u8);
            line_control.write(0x80u8);
            buffer.write(0x03u8);
            interrupt_enable.write(0x00u8);
            line_control.write(0x03u8);
            fifo_control.write(0xC7u8);
            modem_control.write(0x0Bu8);
            modem_control.write(0x1Eu8);
            buffer.write(0xAEu8);
        
            if buffer.read() != 0xAEu8 {
                panic!("Serial port initialization failed");
            }

            modem_control.write(0x0Fu8);
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

#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use esp_hal::{
    clock::CpuClock, main
};

// use esp_hal_smartled::{smart_led_buffer, SmartLedsAdapter};
use esp_println as _;
use defmt::info;

use esp_backtrace as _;
// use smart_leds::{RGB8, SmartLedsWrite as _};

use embedded_io::Read as _;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();


#[main]
fn main() -> ! {
    // Clocks + peripherals
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let p = esp_hal::init(config);

    // ----- USB side (host-facing terminal) -----
    // Open a 115200 terminal on your PC (USB-CDC/JTAG) to watch logs.
    let mut usb = esp_hal::usb_serial_jtag::UsbSerialJtag::new(p.USB_DEVICE);

    // ----- External UART on GPIO16/17 (vape-facing) -----
    // Wire: vape LOG -> GPIO16 (ESP RX), and ESP TX (GPIO17) -> vape candidate RX (KEY or INT).
    let cfg = esp_hal::uart::Config::default().with_baudrate(115_200);
    let mut uart = esp_hal::uart::Uart::new(
        p.UART1,
        cfg,
    )
    .unwrap()
    .with_rx(p.GPIO16)
    .with_tx(p.GPIO17);

    // Small scratch buffers
    let mut usb_to_uart = [0u8; 64];
    let mut uart_to_usb = [0u8; 64];

    info!("UART bridge up: USB<->UART1 @115200, RX=GPIO16, TX=GPIO17");

    // -------- USB -> UART --------
    loop {
        // -------- UART -> USB --------
        // Try to pull whatever is available from UART.
        match uart.read(&mut uart_to_usb) {
            Ok(n) if n > 0 => {
                // Write to USB; ignore partial writes (USB host polling will continue next loop)
                let _ = usb.write(&uart_to_usb[..n]);
            }
            _ => { /* no bytes or transient error; continue */ }
        }

        // -------- USB -> UART --------
        match usb.read(&mut usb_to_uart) {
            Ok(n) if n > 0 => {
                let _ = uart.write(&usb_to_uart[..n]);
            }
            _ => { /* no bytes from host right now */ }
        }
    }
}

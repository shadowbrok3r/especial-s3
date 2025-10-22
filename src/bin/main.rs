#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use esp_hal::{
    clock::CpuClock, main, time::{Duration, Instant}
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
    let mut usb = esp_hal::usb_serial_jtag::UsbSerialJtag::new(p.USB_DEVICE);

    // ----- External UART on GPIO16/17 (vape-facing) -----
    let cfg = esp_hal::uart::Config::default().with_baudrate(115_200);
    let mut uart = esp_hal::uart::Uart::new(
        p.UART0,
        cfg,
    )
    .unwrap()
    .with_rx(p.GPIO44)
    .with_tx(p.GPIO43);

    // Small scratch buffers
    // let mut usb_to_uart = [0u8; 64];
    // let mut uart_to_usb = [0u8; 64];

    info!("UART bridge up: USB<->UART1 @115200, RX=GPIO44, TX=GPIO43");

    let mut last_ping = Instant::now();
    let mut rx = [0u8; 128];

    loop {
        // UART -> USB and echo back to UART (proves both directions)
        if let Ok(n) = uart.read(&mut rx) {
            if n > 0 {
                let _ = usb.write(b"[UART RX] ");
                let _ = usb.write(&rx[..n]);
                let _ = usb.write(b"\r\n");
                // Echo to Flipper so you see it there too
                let _ = uart.write(&rx[..n]);
            }
        }

        // USB -> UART (optional host typing passthrough)
        let mut host_in = [0u8; 64];
        if let Ok(m) = usb.read(&mut host_in) {
            if m > 0 {
                let _ = uart.write(&host_in[..m]);
            }
        }

        // Heartbeat to Flipper every 500 ms.
        if last_ping.elapsed() >= Duration::from_millis(500) {
            let _ = uart.write(b"PING 55 AA\r\n");
            last_ping = Instant::now();
        }
    }
}

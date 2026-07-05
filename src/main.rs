#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

mod wifi;
mod display;
mod icons;

use embassy_net::{Stack, Runner};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::timer::timg::TimerGroup;
use rtt_target::rprintln;
use esp_radio::wifi::{
    Config,
    Interface,
    WifiController,
    scan::ScanConfig,
    sta::StationConfig,
};

// epd
use epd_waveshare::prelude::WaveshareDisplay;

// SPI
use esp_hal::spi;
use esp_hal::spi::master::Spi;
use esp_hal::time::Rate;

use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};

// embedded graphics
use crate::display::setup_display;

#[panic_handler]
fn panic(panic_info: &core::panic::PanicInfo) -> ! {
    rprintln!("{}", panic_info);
    loop {}
}

extern crate alloc;

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");


// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();


#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    rtt_target::rtt_init_print!();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // The following pins are used to bootstrap the chip. They are available
    // for use, but check the datasheet of the module for more information on them.
    // - GPIO4
    // - GPIO5
    // - GPIO8
    // - GPIO9
    // - GPIO15
    // These GPIO pins are in use by some feature of the module and should not be used.
    let _ = peripherals.GPIO24;
    let _ = peripherals.GPIO25;
    let _ = peripherals.GPIO26;
    let _ = peripherals.GPIO27;
    let _ = peripherals.GPIO28;
    let _ = peripherals.GPIO29;
    let _ = peripherals.GPIO30;

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 65536);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    rprintln!("scheduler started");

    let station_config = Config::Station(
        StationConfig::default()
            .with_ssid(SSID)
            .with_password(PASSWORD.into()),
    );

    let (mut wifi_controller, stack, runner ) = wifi::setup_network(peripherals.WIFI, station_config);

    rprintln!("scanning SSIDs");
    let scan_config = ScanConfig::default().with_max(10);
    let result = wifi_controller.scan_async(&scan_config).await.unwrap();
    rprintln!("found SSIDs:");
    for ap in result {
        rprintln!("{:?}", ap);
    }

    spawner.spawn(connection(wifi_controller).unwrap());
    spawner.spawn(net_task(runner).unwrap());


    // Setup display dependencies
    let spi_bus = Spi::new(
        peripherals.SPI2,
        spi::master::Config::default()
            .with_frequency(Rate::from_mhz(4))
            .with_mode(spi::Mode::_0),
    )
        .unwrap()
        //CLK
        .with_sck(peripherals.GPIO18)
        //DIN
        .with_mosi(peripherals.GPIO23);

    let cs = Output::new(peripherals.GPIO10, Level::Low, OutputConfig::default());
    let busy_in = Input::new(
        peripherals.GPIO22,
        InputConfig::default().with_pull(Pull::None),
    );
    let dc = Output::new(peripherals.GPIO17, Level::Low, OutputConfig::default());
    let reset = Output::new(peripherals.GPIO16, Level::Low, OutputConfig::default());

    // Initialize Display
    let mut display = setup_display(spi_bus, cs, busy_in, dc, reset);
    display.draw_text("hello potato", 0, 55);
    display.draw_icon("nights_stay.bmp", 0, 0);
    display.flush().unwrap();

    stack.wait_config_up().await;
    if let Some(config) = stack.config_v4() {
        rprintln!("connected to wifi, got IP: {}", config.address);
    }

    loop {
        rprintln!("Hello world!");
        Timer::after(Duration::from_secs(1)).await;
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.1.0/examples
}

async fn wait_for_connection(stack: Stack<'_>) {
    rprintln!("Waiting for link to be up");
    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    rprintln!("Waiting to get IP address...");
    loop {
        if let Some(config) = stack.config_v4() {
            rprintln!("Got IP: {}", config.address);
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }
}


#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    rprintln!("start connection task");

    loop {
        rprintln!("About to connect...");

        match controller.connect_async().await {
            Ok(info) => {
                rprintln!("Wifi connected to {:?}", info);

                // wait until we're no longer connected
                let info = controller.wait_for_disconnect_async().await.ok();
                rprintln!("Disconnected: {:?}", info);
            }
            Err(e) => {
                rprintln!("Failed to connect to wifi: {e:?}");
            }
        }

        Timer::after(Duration::from_millis(5000)).await
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, Interface<'static>>) {
    runner.run().await
}
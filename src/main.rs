#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

mod display;
mod icons;
mod weatherapi;
mod wifi;

use alloc::string::{String, ToString};
use embassy_executor::Spawner;
use embassy_net::{Runner, Stack};
use embassy_time::{Duration, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::timer::timg::TimerGroup;
use esp_radio::wifi::{Config, Interface, WifiController, scan::ScanConfig, sta::StationConfig};
use rtt_target::rprintln;

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
const API_KEY: &str = env!("API_KEY");

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

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

    esp_alloc::heap_allocator!(size: 160*1024);

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

    let (mut wifi_controller, stack, runner) =
        wifi::setup_network(peripherals.WIFI, station_config);

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

    stack.wait_config_up().await;
    if let Some(config) = stack.config_v4() {
        rprintln!("connected to wifi, got IP: {}", config.address);
    }

    let mut weather_client = weatherapi::new_client(stack, API_KEY);

    loop {
        let mut ok = false;

        while !ok {
            match weather_client.get_forecast().await {
                Ok(forecast) => {
                    ok = true;
                    rprintln!("today's temp is {}", forecast.current.temp_c);

                    let dashboard = display::Dashboard::from_weather_data(&forecast);

                    display.clear();
                    display.draw_dashboard(dashboard);
                    display.flush().unwrap();
                }
                Err(err) => {
                    rprintln!("failed to get weather data: {:?}", err);
                    Timer::after(Duration::from_secs(10)).await;
                }
            }
        }

        Timer::after(Duration::from_secs(3600)).await;
    }
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    rprintln!("start connection task");

    loop {
        rprintln!("trying to connect...");

        match controller.connect_async().await {
            Ok(info) => {
                rprintln!("connected to {:?}", info);

                let info = controller.wait_for_disconnect_async().await.ok();
                rprintln!("disconnected from {:?}", info);
            }
            Err(e) => {
                rprintln!("failed to connect to wifi: {:?}", e);
            }
        }

        Timer::after(Duration::from_secs(10)).await
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, Interface<'static>>) {
    runner.run().await
}

use esp_hal::peripherals::{Peripherals, WIFI};
use esp_radio::wifi::{Config, ControllerConfig, Interfaces, WifiController};
use esp_radio::wifi::sta::StationConfig;
use embassy_net::{DhcpConfig, StackResources, Stack, Runner};

macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}


pub fn setup_network(wifi: WIFI, station_config: Config) -> (WifiController, Stack, Runner<esp_radio::wifi::Interface>)   {
    let (wifi_controller, interfaces) =
        esp_radio::wifi::new(wifi, ControllerConfig::default().with_initial_config(station_config))
            .expect("Failed to initialize Wi-Fi controller");

    let rng = esp_hal::rng::Rng::new();
    let net_seed = rng.random() as u64 | ((rng.random() as u64) << 32);

    let dhcp_config = DhcpConfig::default();
    // dhcp_config.hostname = Some(String::from_str("implRust").unwrap());

    let config = embassy_net::Config::dhcpv4(dhcp_config);
    // Init network stack
    let (stack, runner) = embassy_net::new(
        interfaces.station,
        config,
        mk_static!(StackResources<3>, StackResources::<3>::new()),
        net_seed,
    );


    return (wifi_controller, stack, runner);
}
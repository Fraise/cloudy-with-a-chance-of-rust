pub mod condition_icons;

pub use condition_icons::{icon_for, icon_for_with_day};

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;
use embassy_net::Stack;
use embassy_net::dns::DnsSocket;
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use reqwless::client::{HttpClient, TlsConfig};
use reqwless::request::Method;
use reqwless::response::{Status, StatusCode};
use rtt_target::rprintln;
use serde::Deserialize;

macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

pub struct WeatherAPIClient<'a> {
    http_client: HttpClient<'a, TcpClient<'a, 1, 1500, 1500>, DnsSocket<'a>>,
    api_key: &'a str,
}

pub fn new_client(stack: Stack<'static>, api_key: &'static str) -> WeatherAPIClient<'static> {
    let tcp_client = mk_static!(
        TcpClient<'static, 1, 1500, 1500>,
        TcpClient::new(
            stack,
            mk_static!(
                TcpClientState<1, 1500, 1500>,
                TcpClientState::<1, 1500, 1500>::new()
            ),
        )
    );
    let dns_client = mk_static!(DnsSocket<'static>, DnsSocket::new(stack));

    let rx_buf = mk_static!([u8; 24 * 1024], [0; 24 * 1024]);
    let tx_buf = mk_static!([u8; 24 * 1024], [0; 24 * 1024]);
    let rng = esp_hal::rng::Rng::new();
    let tls_seed = rng.random() as u64 | ((rng.random() as u64) << 32);

    let tls_config = TlsConfig::new(tls_seed, rx_buf, tx_buf, reqwless::client::TlsVerify::None);

    let http_client = HttpClient::new_with_tls(tcp_client, dns_client, tls_config);

    WeatherAPIClient {
        http_client,
        api_key,
    }
}

impl<'a> WeatherAPIClient<'a> {
    pub async fn get_forecast(&mut self) -> Result<WeatherData, WeatherError> {
        let mut rx_buf = [0u8; 64 * 1024];

        let mut url =
            "https://api.weatherapi.com/v1/forecast.json?q=Montreal&days=1&aqi=no&alerts=no&key="
                .to_string();
        url.push_str(self.api_key);

        let mut builder = self.http_client.request(Method::GET, url.as_str()).await?;

        let response = builder.send(&mut rx_buf).await?;

        if !response.status.is_successful() {
            return Err(WeatherError::HttpStatus(response.status));
        }

        let resp_body = response.body().read_to_end().await?;

        serde_json::from_slice::<WeatherData>(&resp_body).map_err(WeatherError::Json)
    }
}

#[derive(Deserialize)]
pub struct HourForecast {
    pub time_epoch: i64,
    pub time: String,
    pub temp_c: f64,
    pub temp_f: f64,
    pub is_day: i64,
    pub condition: Condition,
    pub wind_mph: f64,
    pub wind_kph: f64,
    pub wind_degree: i64,
    pub wind_dir: String,
    pub pressure_mb: f64,
    pub pressure_in: f64,
    pub precip_mm: f64,
    pub precip_in: f64,
    pub snow_cm: f64,
    pub humidity: i64,
    pub cloud: i64,
    pub feelslike_c: f64,
    pub feelslike_f: f64,
    pub windchill_c: f64,
    pub windchill_f: f64,
    pub heatindex_c: f64,
    pub heatindex_f: f64,
    pub dewpoint_c: f64,
    pub dewpoint_f: f64,
    pub will_it_rain: i64,
    pub chance_of_rain: i64,
    pub will_it_snow: i64,
    pub chance_of_snow: i64,
    pub vis_km: f64,
    pub vis_miles: f64,
    pub gust_mph: f64,
    pub gust_kph: f64,
    // pub uv: _,
    // pub short_rad: _,
    // pub diff_rad: _,
    // pub dni: _,
    // pub gti: _,
}

#[derive(Deserialize)]
pub struct Astro {
    pub sunrise: String,
    pub sunset: String,
    pub moonrise: String,
    pub moonset: String,
    pub moon_phase: String,
    pub moon_illumination: i64,
    pub is_moon_up: i64,
    pub is_sun_up: i64,
}

#[derive(Deserialize)]
pub struct Day {
    pub maxtemp_c: f64,
    pub maxtemp_f: f64,
    pub mintemp_c: f64,
    pub mintemp_f: f64,
    pub avgtemp_c: f64,
    pub avgtemp_f: f64,
    pub maxwind_mph: f64,
    pub maxwind_kph: f64,
    pub totalprecip_mm: f64,
    pub totalprecip_in: f64,
    pub totalsnow_cm: f64,
    pub avgvis_km: f64,
    pub avgvis_miles: f64,
    pub avghumidity: i64,
    pub daily_will_it_rain: i64,
    pub daily_chance_of_rain: i64,
    pub daily_will_it_snow: i64,
    pub daily_chance_of_snow: i64,
    pub condition: Condition,
    pub uv: f64,
}

#[derive(Deserialize)]
pub struct DayForecast {
    pub date: String,
    pub date_epoch: i64,
    pub day: Day,
    pub astro: Astro,
    pub hour: Vec<HourForecast>,
}

#[derive(Deserialize)]
pub struct Forecast {
    pub forecastday: Vec<DayForecast>,
}

#[derive(Deserialize)]
pub struct Condition {
    pub text: String,
    pub icon: String,
    pub code: i64,
}

#[derive(Deserialize)]
pub struct Current {
    pub last_updated_epoch: i64,
    pub last_updated: String,
    pub temp_c: f64,
    pub temp_f: f64,
    pub is_day: i64,
    pub condition: Condition,
    pub wind_mph: f64,
    pub wind_kph: f64,
    pub wind_degree: i64,
    pub wind_dir: String,
    pub pressure_mb: f64,
    pub pressure_in: f64,
    pub precip_mm: f64,
    pub precip_in: f64,
    pub humidity: i64,
    pub cloud: i64,
    pub feelslike_c: f64,
    pub feelslike_f: f64,
    pub windchill_c: f64,
    pub windchill_f: f64,
    pub heatindex_c: f64,
    pub heatindex_f: f64,
    pub dewpoint_c: f64,
    pub dewpoint_f: f64,
    pub vis_km: f64,
    pub vis_miles: f64,
    pub uv: f64,
    pub gust_mph: f64,
    pub gust_kph: f64,
    pub will_it_rain: i64,
    pub chance_of_rain: i64,
    pub will_it_snow: i64,
    pub chance_of_snow: i64,
    pub short_rad: f64,
    pub diff_rad: f64,
    pub dni: f64,
    pub gti: f64,
}
#[derive(Deserialize)]
pub struct Location {
    pub name: String,
    pub region: String,
    pub country: String,
    pub lat: f64,
    pub lon: f64,
    pub tz_id: String,
    pub localtime_epoch: i64,
    pub localtime: String,
}

#[derive(Deserialize)]
pub struct WeatherData {
    pub location: Location,
    pub current: Current,
    pub forecast: Forecast,
}

#[derive(Debug)]
pub enum WeatherError {
    Http(reqwless::Error), // network/HTTP failure
    #[allow(dead_code)]
    HttpStatus(StatusCode),
    Json(serde_json::Error), // deserialization failure
}

impl From<reqwless::Error> for WeatherError {
    fn from(e: reqwless::Error) -> Self {
        WeatherError::Http(e)
    }
}

impl From<serde_json::Error> for WeatherError {
    fn from(e: serde_json::Error) -> Self {
        WeatherError::Json(e)
    }
}

impl fmt::Display for WeatherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WeatherError::Http(e) => write!(f, "http error: {e:?}"),
            WeatherError::HttpStatus(_) => write!(f, "http status: {:?}", self),
            WeatherError::Json(e) => write!(f, "json error: {e}"),
        }
    }
}

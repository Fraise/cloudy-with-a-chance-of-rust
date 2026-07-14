use crate::weatherapi::{WeatherData, icon_for};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use embassy_time::Delay;
use embedded_graphics::mono_font::ascii::{FONT_9X15, FONT_8X13};
use embedded_graphics::mono_font::{MonoFont, MonoTextStyleBuilder};
use embedded_graphics::pixelcolor::Rgb555;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Alignment, Text};
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::spi::{ErrorType, SpiBus};
use embedded_hal_bus::spi::ExclusiveDevice;
use epd_waveshare::color::Color;
use epd_waveshare::epd2in13_v2::{Display2in13, Epd2in13};
use epd_waveshare::graphics::DisplayRotation;
use epd_waveshare::prelude::WaveshareDisplay;
use esp_hal::gpio::{Input, Output};
use esp_hal::spi::master::Spi;
use esp_hal::{Blocking, spi};
use tinybmp::Bmp;

/// Concrete alias for the SPI error type our `ExclusiveDevice` produces.
/// Lets the public method signatures stay short.
type SpiError<'a, Bus> = <ExclusiveDevice<Bus, Output<'a>, Delay> as ErrorType>::Error;

/// High-level wrapper that owns the EPD driver, the framebuffer, and the SPI
/// bus, and exposes a small, ergonomic surface area for drawing on and pushing
/// frames to the physical display.
///
/// All of the generic peripherals are owned so the caller never has to thread
/// them through every call site — they call `flush()`, `clear()`, `sleep()`,
/// etc. instead.
pub struct Display<'a, Bus, Busy, Dc, Rst> {
    spi_dev: ExclusiveDevice<Bus, Output<'a>, Delay>,
    epd: Epd2in13<ExclusiveDevice<Bus, Output<'a>, Delay>, Busy, Dc, Rst, Delay>,
    framebuffer: Display2in13,
}

impl<'a, Bus, Busy, Dc, Rst> Display<'a, Bus, Busy, Dc, Rst>
where
    Bus: SpiBus,
    Busy: InputPin,
    Dc: OutputPin,
    Rst: OutputPin,
{
    /// Build the display from a fully-initialised EPD and its framebuffer.
    pub fn new(
        spi_dev: ExclusiveDevice<Bus, Output<'a>, Delay>,
        epd: Epd2in13<ExclusiveDevice<Bus, Output<'a>, Delay>, Busy, Dc, Rst, Delay>,
        framebuffer: Display2in13,
    ) -> Self {
        Self {
            spi_dev,
            epd,
            framebuffer,
        }
    }

    /// Mutable access to the embedded-graphics `DrawTarget` so callers can use
    /// any `Drawable` (text, primitives, images, etc.) to compose a frame.
    pub fn framebuffer(&mut self) -> &mut Display2in13 {
        &mut self.framebuffer
    }

    /// Push the current framebuffer contents to the panel and trigger a
    /// full refresh.
    pub fn flush(&mut self) -> Result<(), SpiError<'a, Bus>> {
        self.epd
            .update_and_display_frame(&mut self.spi_dev, self.framebuffer.buffer(), &mut Delay)
    }

    /// Clear both the EPD's internal frame and the local framebuffer, then
    /// push the cleared image to the panel.
    pub fn clear(&mut self) {
        self.epd.clear_frame(&mut self.spi_dev, &mut Delay).unwrap();
        // `DrawTarget::clear` is `Infallible` for this framebuffer, so the
        // `.unwrap()` never panics in practice. If a future framebuffer adds
        // a real error type, swap this for a `map_err`.
        self.framebuffer.clear(Color::White).unwrap();
    }

    /// Set the rotation used when rendering into the framebuffer.
    fn set_rotation(&mut self, rotation: DisplayRotation) {
        self.framebuffer.set_rotation(rotation);
    }

    /// Put the EPD into deep sleep. Call `wake()` before drawing again.
    pub fn sleep(&mut self) -> Result<(), SpiError<'a, Bus>> {
        self.epd.sleep(&mut self.spi_dev, &mut Delay)
    }

    /// Wake the EPD back up after `sleep()`.
    pub fn wake(&mut self) -> Result<(), SpiError<'a, Bus>> {
        self.epd.wake_up(&mut self.spi_dev, &mut Delay)
    }

    /// Draw a left aligned text in the display buffer.
    pub fn draw_text(&mut self, text: &str, font: MonoFont, x: i32, y: i32) {
        let text_style = MonoTextStyleBuilder::new()
            .font(&font)
            .text_color(Color::Black)
            .build();

        Text::with_alignment(text, Point::new(x, y), text_style, Alignment::Left)
            .draw(&mut self.framebuffer)
            .unwrap();
    }

    pub fn draw_icon(&mut self, icon_name: &str, x: i32, y: i32) {
        let img_bytes = crate::icons::ICONS
            .iter()
            .find(|(name, _)| *name == icon_name)
            .map(|(_, img_bytes)| *img_bytes)
            .expect("unknown icon");

        let bmp: Bmp<Rgb555> = Bmp::from_slice(img_bytes).unwrap();
        let height = bmp.size().height as i32;

        for pixel in bmp.pixels() {
            let (point, rgb) = (pixel.0, pixel.1);
            let luma = (rgb.r() as u32 + rgb.g() as u32 + rgb.b() as u32) / 3;
            let color = if luma > 0x10 {
                Color::White
            } else {
                Color::Black
            };
            let mirrored = Point::new(point.x, height - 1 - point.y);
            let _ = Pixel(Point::new(x, y) + mirrored, color).draw(&mut self.framebuffer);
        }
    }

    pub fn draw_dashboard(&mut self, dashboard: Dashboard) {
        self.draw_text(
            format!("{}", dashboard.day_forecast).as_str(),
            FONT_9X15,
            0,
            10,
        );
        self.draw_text(
            format!("min: {}", dashboard.min_temp).as_str(),
            FONT_9X15,
            0,
            24,
        );
        self.draw_text(
            format!("max: {}", dashboard.max_temp).as_str(),
            FONT_9X15,
            0,
            36,
        );

        if dashboard.total_snow_cm > 0.0 {
            self.draw_text(
                format!("{}% snow", dashboard.chance_of_snow).as_str(),
                FONT_9X15,
                120,
                24,
            );
            self.draw_text(
                format!("{}cm", dashboard.total_snow_cm).as_str(),
                FONT_9X15,
                120,
                36,
            );
        } else {
            self.draw_text(
                format!("{}% rain", dashboard.chance_of_rain).as_str(),
                FONT_9X15,
                120,
                24,
            );
            self.draw_text(
                format!("{}mm", dashboard.total_rain_mm).as_str(),
                FONT_9X15,
                120,
                36,
            );
        }

        self.draw_icon(dashboard.icon_name.as_str(), 204, -2);

        let mut offset = 4;
        for h in dashboard.next_hours.iter() {
            self.draw_text(
                format!("{}", h.time).as_str(),
                FONT_8X13,
                offset,
                54,
            );

            self.draw_text(
                format!("{}C", h.temp).as_str(),
                FONT_8X13,
                offset,
                64,
            );

            if h.snow_cm > 0.0 {
                self.draw_text(
                    format!("{}cm", h.snow_cm as i64).as_str(),
                    FONT_8X13,
                    offset,
                    74,
                );
            } else {
                self.draw_text(
                    format!("{}mm", h.rain_mm as i64).as_str(),
                    FONT_8X13,
                    offset,
                    74,
                );
            }

            self.draw_icon(dashboard.icon_name.as_str(), offset-4, 76);
            offset += 50;
        }
    }
}

/// Initialize the SPI bus, GPIO pins, EPD driver, and framebuffer, and return
/// a ready-to-use `Display`. The concrete peripheral types are hidden behind
/// `impl Trait` so callers don't have to name them.
///
/// The `Display` borrows the chip-select `Output` for as long as it lives, so
/// the `'a` lifetime on the return type is tied to the lifetime of the `cs`
/// parameter — not hardcoded to `'static`. Hardcoding `'static` would only
/// compile if every input pin were also `'static`, which they aren't when
/// they're owned by the caller.
pub fn setup_display<'a>(
    spi_bus: Spi<'static, Blocking>,
    cs: Output<'a>,
    busy_in: Input,
    dc: Output,
    reset: Output,
) -> Display<'a, impl SpiBus, impl InputPin, impl OutputPin, impl OutputPin> {
    let mut spi_dev: ExclusiveDevice<_, _, Delay> =
        ExclusiveDevice::new(spi_bus, cs, Delay).unwrap();

    let mut framebuffer = Display2in13::default();
    framebuffer.set_rotation(DisplayRotation::Rotate90);

    let epd = Epd2in13::new(&mut spi_dev, busy_in, dc, reset, &mut Delay, None).unwrap();

    let mut display = Display::new(spi_dev, epd, framebuffer);

    display.set_rotation(DisplayRotation::Rotate90);
    display.clear();

    display
}

pub struct Dashboard {
    day_forecast: String,
    icon_name: String,
    last_update: String,
    min_temp: f64,
    max_temp: f64,
    chance_of_rain: i64,
    total_rain_mm: f64,
    chance_of_snow: i64,
    total_snow_cm: f64,
    next_hours: Vec<Hour>,
}

pub struct Hour {
    time: String,
    temp: f64,
    icon_name: String,
    rain_mm: f64,
    snow_cm: f64,
}

impl Dashboard {
    pub fn from_weather_data(weather_data: &WeatherData) -> Self {
        let mut icon_name = icon_for(
            weather_data
                .forecast
                .forecastday
                .first()
                .unwrap()
                .day
                .condition
                .code,
        )
        .to_string();
        icon_name.push_str(".bmp");

        let mut next_hours: Vec<Hour> = Vec::new();
        let mut max_hours = 5;

        for h in weather_data.forecast.forecastday.first().unwrap().hour.iter() {
            if max_hours == 0 {
                break;
            }

            if h.time_epoch >= weather_data.location.localtime_epoch {
                let mut ic = icon_for(h.condition.code).to_string();
                ic.push_str(".bmp");

                next_hours.push(Hour{
                    time: h.time.clone().split(" ").last().unwrap().to_string(),
                    rain_mm: h.precip_mm,
                    temp: h.temp_c,
                    icon_name: ic,
                    snow_cm: h.snow_cm,
                });

                max_hours -= 1;
            }
        }

        Dashboard {
            day_forecast: weather_data
                .forecast
                .forecastday
                .first()
                .unwrap()
                .day.condition.text.clone(),
            icon_name,
            max_temp: weather_data
                .forecast
                .forecastday
                .first()
                .unwrap()
                .day
                .maxtemp_c,
            min_temp: weather_data
                .forecast
                .forecastday
                .first()
                .unwrap()
                .day
                .mintemp_c,
            last_update: weather_data.current.last_updated.clone(),
            chance_of_rain: weather_data
                .forecast
                .forecastday
                .first()
                .unwrap()
                .day
                .daily_chance_of_rain,
            total_rain_mm: weather_data
                .forecast
                .forecastday
                .first()
                .unwrap()
                .day
                .totalprecip_mm,
            chance_of_snow: weather_data
                .forecast
                .forecastday
                .first()
                .unwrap()
                .day
                .daily_chance_of_snow,
            total_snow_cm: weather_data
                .forecast
                .forecastday
                .first()
                .unwrap()
                .day
                .totalsnow_cm,
            next_hours,
        }
    }
}

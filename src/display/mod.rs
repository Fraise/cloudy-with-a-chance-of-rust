use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Alignment, Text};
use embassy_time::Delay;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::spi::{ErrorType, SpiBus};
use embedded_hal_bus::spi::ExclusiveDevice;
use epd_waveshare::color::Color;
use epd_waveshare::epd2in13_v2::{Display2in13, Epd2in13};
use epd_waveshare::graphics::DisplayRotation;
use epd_waveshare::prelude::WaveshareDisplay;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::peripherals::Peripherals;
use esp_hal::{spi, Blocking};
use esp_hal::spi::master::Spi;
use esp_hal::time::Rate;

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
    pub fn clear(&mut self) -> Result<(), SpiError<'a, Bus>> {
        self.epd.clear_frame(&mut self.spi_dev, &mut Delay)?;
        // `DrawTarget::clear` is `Infallible` for this framebuffer, so the
        // `.unwrap()` never panics in practice. If a future framebuffer adds
        // a real error type, swap this for a `map_err`.
        self.framebuffer.clear(Color::White).unwrap();
        self.flush()
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
    pub fn draw_text(&mut self, text: &str, x: i32, y: i32) {
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_10X20)
            .text_color(Color::Black)
            .build();

        Text::with_alignment(text, Point::new(x, y), text_style, Alignment::Left).draw(&mut self.framebuffer).unwrap();
    }
}

/// Draw a left-aligned string at `(x, y)` in the default 10x20 font.
///
/// This is a free helper rather than a method on `Display` because it doesn't
/// need any of the EPD/SPI state — it only needs the framebuffer. Keeping it
/// free makes the wrapper easier to test and avoids generic-method noise.
pub fn draw_text(display: &mut Display2in13, text: &str, x: i32, y: i32) {
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_10X20)
        .text_color(Color::Black)
        .build();

    Text::with_alignment(text, Point::new(x, y), text_style, Alignment::Left)
        .draw(display)
        .unwrap();
}

/// Initialise the SPI bus, GPIO pins, EPD driver, and framebuffer, and return
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
) -> Display<
    'a,
    impl SpiBus,
    impl InputPin,
    impl OutputPin,
    impl OutputPin,
> {
    let mut spi_dev: ExclusiveDevice<_, _, Delay> =
        ExclusiveDevice::new(spi_bus, cs, Delay).unwrap();

    let mut framebuffer = Display2in13::default();
    framebuffer.set_rotation(DisplayRotation::Rotate90);

    let epd = Epd2in13::new(&mut spi_dev, busy_in, dc, reset, &mut Delay, None).unwrap();

    let mut display = Display::new(spi_dev, epd, framebuffer);

    display.set_rotation(DisplayRotation::Rotate90);
    display.clear().unwrap();

    display
}

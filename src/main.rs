use anyhow::{Error, Ok};
use embedded_graphics::geometry::Size;
use embedded_graphics::mono_font::{iso_8859_16::*, MonoTextStyle};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics::text::*;
use embedded_graphics::*;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::{AnyOutputPin, IOPin, Output, OutputPin, PinDriver};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::spi::config::DriverConfig;
use esp_idf_hal::spi::*;
use esp_idf_hal::units::Hertz;

mod round_screen;
use round_screen::RoundScreen;

use crate::round_screen::Rotation;

fn main() {
    let peripherals = Peripherals::take().unwrap();
    let configuration = DriverConfig::new();
    let myspi = SpiDriver::new(
        peripherals.spi2,
        peripherals.pins.gpio1,                 //CLK
        peripherals.pins.gpio2,                 //MOSI
        None::<esp_idf_hal::gpio::AnyInputPin>, //MISO
        &configuration,
    )
    .unwrap();
    let spi_config = SpiConfig::new().baudrate(Hertz(40000000));
    let device = SpiDeviceDriver::new(myspi, Some(peripherals.pins.gpio6), &spi_config).unwrap();
    let dc = PinDriver::output(peripherals.pins.gpio3.downgrade_output()).unwrap();
    let cs = PinDriver::output(peripherals.pins.gpio4.downgrade_output()).unwrap();
    let rst = PinDriver::output(peripherals.pins.gpio5.downgrade_output()).unwrap();

    let mut screen = RoundScreen::new(device, dc, cs, rst);
    screen.init_sequence().unwrap();

    screen.fill_screen(0xff00).unwrap();
    screen.set_pixel(100, 100, 0x1F).unwrap();
    FreeRtos::delay_ms(1000);
    // clear the screen first
    screen.set_rotation(Rotation::Rotation0degres).unwrap();
    screen.fill_screen(0x0000).unwrap(); // black

    // --- draw rectangle ---
    let rect_style = PrimitiveStyleBuilder::new()
        .fill_color(Rgb565::CSS_CYAN)
        .build();

    Rectangle::new(Point::new(75, 75), Size::new(60, 40))
        .into_styled(rect_style)
        .draw(&mut screen)
        .unwrap();

    // --- draw single red pixel ---
    screen
        .set_pixel(120, 120, Rgb565::RED.into_storage())
        .unwrap();

    // --- draw text ---
    let text_style = MonoTextStyle::new(&FONT_6X10, Rgb565::CSS_HOT_PINK);

    Text::new("CDFR 2026 !!! <3 ", Point::new(20, 100), text_style)
        .draw(&mut screen)
        .unwrap();
}

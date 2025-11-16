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

struct RoundScreen<'a> {
    spi_device_driver: SpiDeviceDriver<'a, SpiDriver<'a>>,
    dc: PinDriver<'a, AnyOutputPin, Output>,
    cs: PinDriver<'a, AnyOutputPin, Output>,
    rst: PinDriver<'a, AnyOutputPin, Output>,
}
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
    screen.initSequence();

    screen.fill_screen(0xff00);
    screen.set_pixel(100, 100, 0x1F).unwrap();
    FreeRtos::delay_ms(200);
    // clear the screen first
    screen.fill_screen(0x0000); // black

    // --- draw rectangle ---
    let rect_style = PrimitiveStyleBuilder::new()
        .fill_color(Rgb565::WHITE)
        .build();

    Rectangle::new(Point::new(10, 10), Size::new(60, 40))
        .into_styled(rect_style)
        .draw(&mut screen)
        .unwrap();

    // --- draw single red pixel ---
    screen
        .set_pixel(120, 120, Rgb565::RED.into_storage())
        .unwrap();

    // --- draw text ---
    let text_style = MonoTextStyle::new(&FONT_6X10, Rgb565::GREEN);

    Text::new("Hello", Point::new(20, 100), text_style)
        .draw(&mut screen)
        .unwrap();
}
impl<'a> Dimensions for RoundScreen<'a> {
    fn bounding_box(&self) -> primitives::Rectangle {
        rectangle::Rectangle::new(
            Point { x: 0, y: 0 },
            Size {
                width: 240,
                height: 240,
            },
        )
    }
}
impl<'a> DrawTarget for RoundScreen<'a> {
    type Color = Rgb565;
    type Error = Error;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let bb = self.bounding_box();
        pixels
            .into_iter()
            .filter(|&Pixel(pos, _color)| bb.contains(pos))
            .try_for_each(|Pixel(pos, color)| {
                let color: pixelcolor::raw::RawU16 = color.into();
                let color: u16 = color.into_inner();
                self.set_pixel(pos.x as u16, pos.y as u16, color)
            })
    }
}

impl<'a> RoundScreen<'a> {
    fn new(
        spi: SpiDeviceDriver<'a, SpiDriver<'a>>,
        dc: PinDriver<'a, AnyOutputPin, Output>,
        cs: PinDriver<'a, AnyOutputPin, Output>,
        rst: PinDriver<'a, AnyOutputPin, Output>,
    ) -> Self {
        RoundScreen {
            spi_device_driver: spi,
            dc,
            cs,
            rst,
        }
    }

    fn sendcmd(&mut self, value: &[u8]) {
        self.dc.set_low().unwrap();
        self.cs.set_low().unwrap();
        self.spi_device_driver.write(value).unwrap();
        self.cs.set_high().unwrap();
    }
    fn senddata(&mut self, value: &[u8]) {
        self.dc.set_high().unwrap();
        self.cs.set_low().unwrap();
        self.spi_device_driver.write(value).unwrap();
        self.cs.set_high().unwrap();
    }
    pub fn set_pixel(&mut self, x: u16, y: u16, color: u16) -> Result<(), anyhow::Error> {
        // Column address
        self.sendcmd(&[0x2A]);
        self.senddata(&[
            (x >> 8) as u8,
            (x & 0xFF) as u8, // start
            (x >> 8) as u8,
            (x & 0xFF) as u8, // end = same pixel
        ]);

        // Row address
        self.sendcmd(&[0x2B]);
        self.senddata(&[
            (y >> 8) as u8,
            (y & 0xFF) as u8,
            (y >> 8) as u8,
            (y & 0xFF) as u8,
        ]);

        // Memory write
        self.sendcmd(&[0x2C]);
        self.senddata(&[(color >> 8) as u8, (color & 0xFF) as u8]);

        Ok(())
    }

    fn fill_screen(&mut self, value: u16) {
        let mut arr = [0x00; 240 * 2];
        for i in 0..240 {
            arr[2 * i] = (value >> 8) as u8;
            arr[2 * i + 1] = (value & 0xff) as u8;
        }
        self.sendcmd(&[0x2a]);
        self.senddata(&[0x00, 0x00, 0x00, 0xef]);
        self.sendcmd(&[0x2b]);
        self.senddata(&[0x00, 0x00, 0x00, 0xef]); //decalage en haut et bas
        self.sendcmd(&[0x2c]);
        for _ in 0..240 {
            self.senddata(&arr);
        }
    }

    fn initSequence(&mut self) {
        FreeRtos::delay_ms(100);
        self.rst.set_low().unwrap();
        FreeRtos::delay_ms(100);
        self.rst.set_high().unwrap(); //reset
        self.cs.set_high().unwrap(); //cs init
        FreeRtos::delay_ms(200);
        //=======================================================
        self.sendcmd(&[0xEF]);
        self.sendcmd(&[0xEB]);
        self.senddata(&[0x14]);

        self.sendcmd(&[0xFE]);
        self.sendcmd(&[0xEF]);

        self.sendcmd(&[0xEB]);
        self.senddata(&[0x14]);

        self.sendcmd(&[0x84]);
        self.senddata(&[0x40]);

        self.sendcmd(&[0x85]);
        self.senddata(&[0xFF]);

        self.sendcmd(&[0x86]);
        self.senddata(&[0xFF]);

        self.sendcmd(&[0x87]);
        self.senddata(&[0xFF]);

        self.sendcmd(&[0x88]);
        self.senddata(&[0x0A]);

        self.sendcmd(&[0x89]);
        self.senddata(&[0x21]);

        self.sendcmd(&[0x8A]);
        self.senddata(&[0x00]);

        self.sendcmd(&[0x8B]);
        self.senddata(&[0x80]);

        self.sendcmd(&[0x8C]);
        self.senddata(&[0x01]);

        self.sendcmd(&[0x8D]);
        self.senddata(&[0x01]);

        self.sendcmd(&[0x8E]);
        self.senddata(&[0xFF]);

        self.sendcmd(&[0x8F]);
        self.senddata(&[0xFF]);

        self.sendcmd(&[0xB6]);
        self.senddata(&[0x00]);
        self.senddata(&[0x20]);

        self.sendcmd(&[0x36]);
        self.senddata(&[0xa8]); //Set as vertical screen

        self.sendcmd(&[0x3A]);
        self.senddata(&[0x05]);

        self.sendcmd(&[0x90]);
        self.senddata(&[0x08]);
        self.senddata(&[0x08]);
        self.senddata(&[0x08]);
        self.senddata(&[0x08]);

        self.sendcmd(&[0xBD]);
        self.senddata(&[0x06]);

        self.sendcmd(&[0xBC]);
        self.senddata(&[0x00]);

        self.sendcmd(&[0xFF]);
        self.senddata(&[0x60]);
        self.senddata(&[0x01]);
        self.senddata(&[0x04]);

        self.sendcmd(&[0xC3]);
        self.senddata(&[0x13]);
        self.sendcmd(&[0xC4]);
        self.senddata(&[0x13]);

        self.sendcmd(&[0xC9]);
        self.senddata(&[0x22]);

        self.sendcmd(&[0xBE]);
        self.senddata(&[0x11]);

        self.sendcmd(&[0xE1]);
        self.senddata(&[0x10]);
        self.senddata(&[0x0E]);

        self.sendcmd(&[0xDF]);
        self.senddata(&[0x21]);
        self.senddata(&[0x0c]);
        self.senddata(&[0x02]);

        self.sendcmd(&[0xF0]);
        self.senddata(&[0x45]);
        self.senddata(&[0x09]);
        self.senddata(&[0x08]);
        self.senddata(&[0x08]);
        self.senddata(&[0x26]);
        self.senddata(&[0x2A]);

        self.sendcmd(&[0xF1]);
        self.senddata(&[0x43]);
        self.senddata(&[0x70]);
        self.senddata(&[0x72]);
        self.senddata(&[0x36]);
        self.senddata(&[0x37]);
        self.senddata(&[0x6F]);

        self.sendcmd(&[0xF2]);
        self.senddata(&[0x45]);
        self.senddata(&[0x09]);
        self.senddata(&[0x08]);
        self.senddata(&[0x08]);
        self.senddata(&[0x26]);
        self.senddata(&[0x2A]);

        self.sendcmd(&[0xF3]);
        self.senddata(&[0x43]);
        self.senddata(&[0x70]);
        self.senddata(&[0x72]);
        self.senddata(&[0x36]);
        self.senddata(&[0x37]);
        self.senddata(&[0x6F]);

        self.sendcmd(&[0xED]);
        self.senddata(&[0x1B]);
        self.senddata(&[0x0B]);

        self.sendcmd(&[0xAE]);
        self.senddata(&[0x77]);

        self.sendcmd(&[0xCD]);
        self.senddata(&[0x63]);

        self.sendcmd(&[0x70]);
        self.senddata(&[0x07]);
        self.senddata(&[0x07]);
        self.senddata(&[0x04]);
        self.senddata(&[0x0E]);
        self.senddata(&[0x0F]);
        self.senddata(&[0x09]);
        self.senddata(&[0x07]);
        self.senddata(&[0x08]);
        self.senddata(&[0x03]);

        self.sendcmd(&[0xE8]);
        self.senddata(&[0x34]);

        self.sendcmd(&[0x62]);
        self.senddata(&[0x18]);
        self.senddata(&[0x0D]);
        self.senddata(&[0x71]);
        self.senddata(&[0xED]);
        self.senddata(&[0x70]);
        self.senddata(&[0x70]);
        self.senddata(&[0x18]);
        self.senddata(&[0x0F]);
        self.senddata(&[0x71]);
        self.senddata(&[0xEF]);
        self.senddata(&[0x70]);
        self.senddata(&[0x70]);

        self.sendcmd(&[0x63]);
        self.senddata(&[0x18]);
        self.senddata(&[0x11]);
        self.senddata(&[0x71]);
        self.senddata(&[0xF1]);
        self.senddata(&[0x70]);
        self.senddata(&[0x70]);
        self.senddata(&[0x18]);
        self.senddata(&[0x13]);
        self.senddata(&[0x71]);
        self.senddata(&[0xF3]);
        self.senddata(&[0x70]);
        self.senddata(&[0x70]);

        self.sendcmd(&[0x64]);
        self.senddata(&[0x28]);
        self.senddata(&[0x29]);
        self.senddata(&[0xF1]);
        self.senddata(&[0x01]);
        self.senddata(&[0xF1]);
        self.senddata(&[0x00]);
        self.senddata(&[0x07]);

        self.sendcmd(&[0x66]);
        self.senddata(&[0x3C]);
        self.senddata(&[0x00]);
        self.senddata(&[0xCD]);
        self.senddata(&[0x67]);
        self.senddata(&[0x45]);
        self.senddata(&[0x45]);
        self.senddata(&[0x10]);
        self.senddata(&[0x00]);
        self.senddata(&[0x00]);
        self.senddata(&[0x00]);

        self.sendcmd(&[0x67]);
        self.senddata(&[0x00]);
        self.senddata(&[0x3C]);
        self.senddata(&[0x00]);
        self.senddata(&[0x00]);
        self.senddata(&[0x00]);
        self.senddata(&[0x01]);
        self.senddata(&[0x54]);
        self.senddata(&[0x10]);
        self.senddata(&[0x32]);
        self.senddata(&[0x98]);

        self.sendcmd(&[0x74]);
        self.senddata(&[0x10]);
        self.senddata(&[0x85]);
        self.senddata(&[0x80]);
        self.senddata(&[0x00]);
        self.senddata(&[0x00]);
        self.senddata(&[0x4E]);
        self.senddata(&[0x00]);

        self.sendcmd(&[0x98]);
        self.senddata(&[0x3e]);
        self.senddata(&[0x07]);

        self.sendcmd(&[0x35]);
        self.sendcmd(&[0x21]);

        self.sendcmd(&[0x11]);
        FreeRtos::delay_ms(120);
        self.sendcmd(&[0x29]);
        FreeRtos::delay_ms(20);
        self.fill_screen(0x001F);
    }
}

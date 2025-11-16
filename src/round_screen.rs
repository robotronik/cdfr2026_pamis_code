use anyhow::*;
use embedded_graphics::geometry::Size;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::{Dimensions, DrawTarget, IntoStorage, Point};
use embedded_graphics::{primitives, Pixel};
use esp_idf_hal::gpio::{AnyOutputPin, Output, PinDriver};
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::spi::{SpiDeviceDriver, SpiDriver};

#[allow(unused)]
pub enum Rotation {
    Rotation0degres,
    Rotation90degres,
    Rotation180degres,
    Rotation270degres,
}

pub struct RoundScreen<'a> {
    spi_device_driver: SpiDeviceDriver<'a, SpiDriver<'a>>,
    dc: PinDriver<'a, AnyOutputPin, Output>,
    cs: PinDriver<'a, AnyOutputPin, Output>,
    rst: PinDriver<'a, AnyOutputPin, Output>,
}

impl<'a> Dimensions for RoundScreen<'a> {
    fn bounding_box(&self) -> primitives::Rectangle {
        primitives::rectangle::Rectangle::new(
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
        let mut buf = [0u8; 240 * 2];
        let mut buf_len = 0usize;

        let mut span_y = 0i32;
        let mut span_x0 = 0i32;
        let mut span_x1 = 0i32;

        let mut last_x = -1312;
        let mut last_y = -1312;

        for Pixel(pos, color) in pixels.into_iter() {
            let x = pos.x as i32;
            let y = pos.y as i32;

            // New span?
            if y != last_y || x != last_x + 1 {
                // Flush old span if it exists
                if buf_len != 0 {
                    self.set_window(span_x0 as u16, span_y as u16, span_x1 as u16, span_y as u16)?;
                    self.senddata(&buf[..buf_len])?;
                    buf_len = 0;
                }

                // Start new span
                span_y = y;
                span_x0 = x;
                span_x1 = x;
            } else {
                span_x1 = x;
            }

            // Push pixel
            let raw = color.into_storage();
            buf[buf_len] = (raw >> 8) as u8;
            buf[buf_len + 1] = raw as u8;
            buf_len += 2;

            last_x = x;
            last_y = y;
        }

        // Flush last span
        if buf_len != 0 {
            self.set_window(span_x0 as u16, span_y as u16, span_x1 as u16, span_y as u16)?;
            self.senddata(&buf[..buf_len])?;
        }

        Ok(())
    }
}

impl<'a> RoundScreen<'a> {
    pub fn new(
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

    pub fn set_rotation(&mut self, rotation: Rotation) -> Result<()> {
        let madctl = match rotation {
            Rotation::Rotation0degres => 0x08,
            Rotation::Rotation90degres => 0x68,
            Rotation::Rotation180degres => 0xC8,
            Rotation::Rotation270degres => 0xA8,
        };

        self.sendcmd(&[0x36])?; // MADCTL register
        self.senddata(&[madctl])?;
        Ok(())
    }

    fn sendcmd(&mut self, value: &[u8]) -> Result<()> {
        self.dc.set_low()?;
        self.cs.set_low()?;
        self.spi_device_driver.write(value)?;
        self.cs.set_high()?;
        Ok(())
    }
    fn senddata(&mut self, value: &[u8]) -> Result<()> {
        self.dc.set_high()?;
        self.cs.set_low()?;
        self.spi_device_driver.write(value)?;
        self.cs.set_high()?;
        Ok(())
    }

    /// Sets the drawing window (column + row address set)
    pub fn set_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) -> Result<(), anyhow::Error> {
        // Column range
        self.sendcmd(&[0x2A])?;
        self.senddata(&[
            (x0 >> 8) as u8,
            (x0 & 0xFF) as u8,
            (x1 >> 8) as u8,
            (x1 & 0xFF) as u8,
        ])?;

        // Row range
        self.sendcmd(&[0x2B])?;
        self.senddata(&[
            (y0 >> 8) as u8,
            (y0 & 0xFF) as u8,
            (y1 >> 8) as u8,
            (y1 & 0xFF) as u8,
        ])?;

        // Write-memory command (no data yet)
        self.sendcmd(&[0x2C])?;

        Ok(())
    }

    pub fn set_pixel(&mut self, x: u16, y: u16, color: u16) -> Result<(), anyhow::Error> {
        // define a 1×1 window
        self.set_window(x, y, x, y)?;

        // push the pixel
        self.senddata(&[(color >> 8) as u8, (color & 0xFF) as u8])?;

        Ok(())
    }

    pub fn fill_screen(&mut self, value: u16) -> Result<()> {
        let mut arr = [0x00; 240 * 2];
        for i in 0..240 {
            arr[2 * i] = (value >> 8) as u8;
            arr[2 * i + 1] = (value & 0xff) as u8;
        }
        self.sendcmd(&[0x2a])?;
        self.senddata(&[0x00, 0x00, 0x00, 0xef])?;
        self.sendcmd(&[0x2b])?;
        self.senddata(&[0x00, 0x00, 0x00, 0xef])?; //decalage en haut et bas
        self.sendcmd(&[0x2c])?;
        for _ in 0..240 {
            self.senddata(&arr)?;
        }
        Ok(())
    }

    pub fn init_sequence(&mut self) -> Result<()> {
        FreeRtos::delay_ms(100);
        self.rst.set_low()?;
        FreeRtos::delay_ms(100);
        self.rst.set_high()?; //reset
        self.cs.set_high()?; //cs init
        FreeRtos::delay_ms(200);
        //=======================================================
        self.sendcmd(&[0xEF])?;
        self.sendcmd(&[0xEB])?;
        self.senddata(&[0x14])?;

        self.sendcmd(&[0xFE])?;
        self.sendcmd(&[0xEF])?;

        self.sendcmd(&[0xEB])?;
        self.senddata(&[0x14])?;

        self.sendcmd(&[0x84])?;
        self.senddata(&[0x40])?;

        self.sendcmd(&[0x85])?;
        self.senddata(&[0xFF])?;

        self.sendcmd(&[0x86])?;
        self.senddata(&[0xFF])?;

        self.sendcmd(&[0x87])?;
        self.senddata(&[0xFF])?;

        self.sendcmd(&[0x88])?;
        self.senddata(&[0x0A])?;

        self.sendcmd(&[0x89])?;
        self.senddata(&[0x21])?;

        self.sendcmd(&[0x8A])?;
        self.senddata(&[0x00])?;

        self.sendcmd(&[0x8B])?;
        self.senddata(&[0x80])?;

        self.sendcmd(&[0x8C])?;
        self.senddata(&[0x01])?;

        self.sendcmd(&[0x8D])?;
        self.senddata(&[0x01])?;

        self.sendcmd(&[0x8E])?;
        self.senddata(&[0xFF])?;

        self.sendcmd(&[0x8F])?;
        self.senddata(&[0xFF])?;

        self.sendcmd(&[0xB6])?;
        self.senddata(&[0x00])?;
        self.senddata(&[0x20])?;

        self.sendcmd(&[0x36])?; //Memory Access Control
        self.senddata(&[0x08])?; //Set RGB

        self.sendcmd(&[0x3A])?;
        self.senddata(&[0x05])?;

        self.sendcmd(&[0x90])?;
        self.senddata(&[0x08])?;
        self.senddata(&[0x08])?;
        self.senddata(&[0x08])?;
        self.senddata(&[0x08])?;

        self.sendcmd(&[0xBD])?;
        self.senddata(&[0x06])?;

        self.sendcmd(&[0xBC])?;
        self.senddata(&[0x00])?;

        self.sendcmd(&[0xFF])?;
        self.senddata(&[0x60])?;
        self.senddata(&[0x01])?;
        self.senddata(&[0x04])?;

        self.sendcmd(&[0xC3])?;
        self.senddata(&[0x13])?;
        self.sendcmd(&[0xC4])?;
        self.senddata(&[0x13])?;

        self.sendcmd(&[0xC9])?;
        self.senddata(&[0x22])?;

        self.sendcmd(&[0xBE])?;
        self.senddata(&[0x11])?;

        self.sendcmd(&[0xE1])?;
        self.senddata(&[0x10])?;
        self.senddata(&[0x0E])?;

        self.sendcmd(&[0xDF])?;
        self.senddata(&[0x21])?;
        self.senddata(&[0x0c])?;
        self.senddata(&[0x02])?;

        self.sendcmd(&[0xF0])?;
        self.senddata(&[0x45])?;
        self.senddata(&[0x09])?;
        self.senddata(&[0x08])?;
        self.senddata(&[0x08])?;
        self.senddata(&[0x26])?;
        self.senddata(&[0x2A])?;

        self.sendcmd(&[0xF1])?;
        self.senddata(&[0x43])?;
        self.senddata(&[0x70])?;
        self.senddata(&[0x72])?;
        self.senddata(&[0x36])?;
        self.senddata(&[0x37])?;
        self.senddata(&[0x6F])?;

        self.sendcmd(&[0xF2])?;
        self.senddata(&[0x45])?;
        self.senddata(&[0x09])?;
        self.senddata(&[0x08])?;
        self.senddata(&[0x08])?;
        self.senddata(&[0x26])?;
        self.senddata(&[0x2A])?;

        self.sendcmd(&[0xF3])?;
        self.senddata(&[0x43])?;
        self.senddata(&[0x70])?;
        self.senddata(&[0x72])?;
        self.senddata(&[0x36])?;
        self.senddata(&[0x37])?;
        self.senddata(&[0x6F])?;

        self.sendcmd(&[0xED])?;
        self.senddata(&[0x1B])?;
        self.senddata(&[0x0B])?;

        self.sendcmd(&[0xAE])?;
        self.senddata(&[0x77])?;

        self.sendcmd(&[0xCD])?;
        self.senddata(&[0x63])?;

        self.sendcmd(&[0x70])?;
        self.senddata(&[0x07])?;
        self.senddata(&[0x07])?;
        self.senddata(&[0x04])?;
        self.senddata(&[0x0E])?;
        self.senddata(&[0x0F])?;
        self.senddata(&[0x09])?;
        self.senddata(&[0x07])?;
        self.senddata(&[0x08])?;
        self.senddata(&[0x03])?;

        self.sendcmd(&[0xE8])?;
        self.senddata(&[0x34])?;

        self.sendcmd(&[0x62])?;
        self.senddata(&[0x18])?;
        self.senddata(&[0x0D])?;
        self.senddata(&[0x71])?;
        self.senddata(&[0xED])?;
        self.senddata(&[0x70])?;
        self.senddata(&[0x70])?;
        self.senddata(&[0x18])?;
        self.senddata(&[0x0F])?;
        self.senddata(&[0x71])?;
        self.senddata(&[0xEF])?;
        self.senddata(&[0x70])?;
        self.senddata(&[0x70])?;

        self.sendcmd(&[0x63])?;
        self.senddata(&[0x18])?;
        self.senddata(&[0x11])?;
        self.senddata(&[0x71])?;
        self.senddata(&[0xF1])?;
        self.senddata(&[0x70])?;
        self.senddata(&[0x70])?;
        self.senddata(&[0x18])?;
        self.senddata(&[0x13])?;
        self.senddata(&[0x71])?;
        self.senddata(&[0xF3])?;
        self.senddata(&[0x70])?;
        self.senddata(&[0x70])?;

        self.sendcmd(&[0x64])?;
        self.senddata(&[0x28])?;
        self.senddata(&[0x29])?;
        self.senddata(&[0xF1])?;
        self.senddata(&[0x01])?;
        self.senddata(&[0xF1])?;
        self.senddata(&[0x00])?;
        self.senddata(&[0x07])?;

        self.sendcmd(&[0x66])?;
        self.senddata(&[0x3C])?;
        self.senddata(&[0x00])?;
        self.senddata(&[0xCD])?;
        self.senddata(&[0x67])?;
        self.senddata(&[0x45])?;
        self.senddata(&[0x45])?;
        self.senddata(&[0x10])?;
        self.senddata(&[0x00])?;
        self.senddata(&[0x00])?;
        self.senddata(&[0x00])?;

        self.sendcmd(&[0x67])?;
        self.senddata(&[0x00])?;
        self.senddata(&[0x3C])?;
        self.senddata(&[0x00])?;
        self.senddata(&[0x00])?;
        self.senddata(&[0x00])?;
        self.senddata(&[0x01])?;
        self.senddata(&[0x54])?;
        self.senddata(&[0x10])?;
        self.senddata(&[0x32])?;
        self.senddata(&[0x98])?;

        self.sendcmd(&[0x74])?;
        self.senddata(&[0x10])?;
        self.senddata(&[0x85])?;
        self.senddata(&[0x80])?;
        self.senddata(&[0x00])?;
        self.senddata(&[0x00])?;
        self.senddata(&[0x4E])?;
        self.senddata(&[0x00])?;

        self.sendcmd(&[0x98])?;
        self.senddata(&[0x3e])?;
        self.senddata(&[0x07])?;

        self.sendcmd(&[0x35])?;
        self.sendcmd(&[0x21])?;

        self.sendcmd(&[0x11])?;
        FreeRtos::delay_ms(120);
        self.sendcmd(&[0x29])?;
        FreeRtos::delay_ms(20);
        self.fill_screen(0x0000)?;
        Ok(())
    }
}

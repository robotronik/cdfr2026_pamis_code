use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::spi::config::DriverConfig;
use esp_idf_hal::spi::*;
use esp_idf_hal::units::Hertz;

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
    let mut device =
        SpiDeviceDriver::new(myspi, Some(peripherals.pins.gpio6), &spi_config).unwrap();
    let mut dc = PinDriver::output(peripherals.pins.gpio3).unwrap();
    let mut cs = PinDriver::output(peripherals.pins.gpio4).unwrap();
    let mut rst = PinDriver::output(peripherals.pins.gpio5).unwrap();
    macro_rules! send {
        (cmd: $val:expr) => {{
            dc.set_low().unwrap();
            cs.set_low().unwrap();
            device.write($val).unwrap();
            cs.set_high().unwrap();
        }};
        (data: $val:expr) => {{
            dc.set_high().unwrap();
            cs.set_low().unwrap();
            device.write($val).unwrap();
            cs.set_high().unwrap();
        }};
    }

    macro_rules! fill_screen {
        ([$a : expr, $b:expr]) => {{
            let mut arr = [0x00;240*2];
            for i in 0..240 {
                arr[2*i] = $a;
                arr[2*i + 1] = $b;
            }
            send!(cmd: &[0x2a]);
            send!(data: &[0x00,0x00,0x00,0xef]);
            send!(cmd: &[0x2b]);
            send!(data: &[0x00,0x00 ,0x00,0xef]); //decalage en haut et bas
            send!(cmd: &[0x2c]);
            for _ in 0..240 {
                send!(data: &arr)
            }
        }};
    }

    macro_rules! initSequence {
        () => {{FreeRtos::delay_ms(100);
    rst.set_low().unwrap();
    FreeRtos::delay_ms(100);
    rst.set_high().unwrap(); //reset
    cs.set_high().unwrap(); //cs init
    FreeRtos::delay_ms(1000);
    //=======================================================
    send!(cmd: &[0xEF]);
    send!(cmd: &[0xEB]);
    send!(data: &[0x14]);

    send!(cmd: &[0xFE]);
    send!(cmd: &[0xEF]);

    send!(cmd: &[0xEB]);
    send!(data: &[0x14]);

    send!(cmd: &[0x84]);
    send!(data: &[0x40]);

    send!(cmd: &[0x85]);
    send!(data: &[0xFF]);

    send!(cmd: &[0x86]);
    send!(data: &[0xFF]);

    send!(cmd: &[0x87]);
    send!(data: &[0xFF]);

    send!(cmd: &[0x88]);
    send!(data: &[0x0A]);

    send!(cmd: &[0x89]);
    send!(data: &[0x21]);

    send!(cmd: &[0x8A]);
    send!(data: &[0x00]);

    send!(cmd: &[0x8B]);
    send!(data: &[0x80]);

    send!(cmd: &[0x8C]);
    send!(data: &[0x01]);

    send!(cmd: &[0x8D]);
    send!(data: &[0x01]);

    send!(cmd: &[0x8E]);
    send!(data: &[0xFF]);

    send!(cmd: &[0x8F]);
    send!(data: &[0xFF]);

    send!(cmd: &[0xB6]);
    send!(data: &[0x00]);
    send!(data: &[0x20]);

    send!(cmd: &[0x36]);
    send!(data: &[0xa8]); //Set as vertical screen

    send!(cmd: &[0x3A]);
    send!(data: &[0x05]);

    send!(cmd: &[0x90]);
    send!(data: &[0x08]);
    send!(data: &[0x08]);
    send!(data: &[0x08]);
    send!(data: &[0x08]);

    send!(cmd: &[0xBD]);
    send!(data: &[0x06]);

    send!(cmd: &[0xBC]);
    send!(data: &[0x00]);

    send!(cmd: &[0xFF]);
    send!(data: &[0x60]);
    send!(data: &[0x01]);
    send!(data: &[0x04]);

    send!(cmd: &[0xC3]);
    send!(data: &[0x13]);
    send!(cmd: &[0xC4]);
    send!(data: &[0x13]);

    send!(cmd: &[0xC9]);
    send!(data: &[0x22]);

    send!(cmd: &[0xBE]);
    send!(data: &[0x11]);

    send!(cmd: &[0xE1]);
    send!(data: &[0x10]);
    send!(data: &[0x0E]);

    send!(cmd: &[0xDF]);
    send!(data: &[0x21]);
    send!(data: &[0x0c]);
    send!(data: &[0x02]);

    send!(cmd: &[0xF0]);
    send!(data: &[0x45]);
    send!(data: &[0x09]);
    send!(data: &[0x08]);
    send!(data: &[0x08]);
    send!(data: &[0x26]);
    send!(data: &[0x2A]);

    send!(cmd: &[0xF1]);
    send!(data: &[0x43]);
    send!(data: &[0x70]);
    send!(data: &[0x72]);
    send!(data: &[0x36]);
    send!(data: &[0x37]);
    send!(data: &[0x6F]);

    send!(cmd: &[0xF2]);
    send!(data: &[0x45]);
    send!(data: &[0x09]);
    send!(data: &[0x08]);
    send!(data: &[0x08]);
    send!(data: &[0x26]);
    send!(data: &[0x2A]);

    send!(cmd: &[0xF3]);
    send!(data: &[0x43]);
    send!(data: &[0x70]);
    send!(data: &[0x72]);
    send!(data: &[0x36]);
    send!(data: &[0x37]);
    send!(data: &[0x6F]);

    send!(cmd: &[0xED]);
    send!(data: &[0x1B]);
    send!(data: &[0x0B]);

    send!(cmd: &[0xAE]);
    send!(data: &[0x77]);

    send!(cmd: &[0xCD]);
    send!(data: &[0x63]);

    send!(cmd: &[0x70]);
    send!(data: &[0x07]);
    send!(data: &[0x07]);
    send!(data: &[0x04]);
    send!(data: &[0x0E]);
    send!(data: &[0x0F]);
    send!(data: &[0x09]);
    send!(data: &[0x07]);
    send!(data: &[0x08]);
    send!(data: &[0x03]);

    send!(cmd: &[0xE8]);
    send!(data: &[0x34]);

    send!(cmd: &[0x62]);
    send!(data: &[0x18]);
    send!(data: &[0x0D]);
    send!(data: &[0x71]);
    send!(data: &[0xED]);
    send!(data: &[0x70]);
    send!(data: &[0x70]);
    send!(data: &[0x18]);
    send!(data: &[0x0F]);
    send!(data: &[0x71]);
    send!(data: &[0xEF]);
    send!(data: &[0x70]);
    send!(data: &[0x70]);

    send!(cmd: &[0x63]);
    send!(data: &[0x18]);
    send!(data: &[0x11]);
    send!(data: &[0x71]);
    send!(data: &[0xF1]);
    send!(data: &[0x70]);
    send!(data: &[0x70]);
    send!(data: &[0x18]);
    send!(data: &[0x13]);
    send!(data: &[0x71]);
    send!(data: &[0xF3]);
    send!(data: &[0x70]);
    send!(data: &[0x70]);

    send!(cmd: &[0x64]);
    send!(data: &[0x28]);
    send!(data: &[0x29]);
    send!(data: &[0xF1]);
    send!(data: &[0x01]);
    send!(data: &[0xF1]);
    send!(data: &[0x00]);
    send!(data: &[0x07]);

    send!(cmd: &[0x66]);
    send!(data: &[0x3C]);
    send!(data: &[0x00]);
    send!(data: &[0xCD]);
    send!(data: &[0x67]);
    send!(data: &[0x45]);
    send!(data: &[0x45]);
    send!(data: &[0x10]);
    send!(data: &[0x00]);
    send!(data: &[0x00]);
    send!(data: &[0x00]);

    send!(cmd: &[0x67]);
    send!(data: &[0x00]);
    send!(data: &[0x3C]);
    send!(data: &[0x00]);
    send!(data: &[0x00]);
    send!(data: &[0x00]);
    send!(data: &[0x01]);
    send!(data: &[0x54]);
    send!(data: &[0x10]);
    send!(data: &[0x32]);
    send!(data: &[0x98]);

    send!(cmd: &[0x74]);
    send!(data: &[0x10]);
    send!(data: &[0x85]);
    send!(data: &[0x80]);
    send!(data: &[0x00]);
    send!(data: &[0x00]);
    send!(data: &[0x4E]);
    send!(data: &[0x00]);

    send!(cmd: &[0x98]);
    send!(data: &[0x3e]);
    send!(data: &[0x07]);

    send!(cmd: &[0x35]);
    send!(cmd: &[0x21]);

    send!(cmd: &[0x11]);
    FreeRtos::delay_ms(120);
    send!(cmd: &[0x29]);
    FreeRtos::delay_ms(20);
    fill_screen!([0x00, 0x1f]);
        }};
    }

    initSequence!();

    //======================================
    println!("0");
    send!(cmd: &[0x2a]);
    send!(data: &[0x00,0x10,0x00,0xef]);

    for i in 0..240 {
        send!(cmd: &[0x2b]);
        send!(data: &[0x00,i ,0x00,0xef]); //decalage en haut et bas
        send!(cmd: &[0x2c]);
        send!(data: &[i;20*2]);
    }
    println!("1");
    FreeRtos::delay_ms(4000);
    send!(cmd: &[0x20]); //inversion on
    FreeRtos::delay_ms(2000);
    send!(cmd: &[0x21]); //inversion off
    FreeRtos::delay_ms(2000);
    fill_screen!([0x00, 0x00]);
    println!("done");

    //
}

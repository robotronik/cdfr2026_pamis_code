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

    FreeRtos::delay_ms(100);
    rst.set_low().unwrap();
    FreeRtos::delay_ms(100);
    rst.set_high().unwrap(); //reset
    cs.set_high().unwrap(); //cs init
    FreeRtos::delay_ms(1000);

    send!(cmd: &[0x11]); //sleep mode off
    FreeRtos::delay_ms(200);
    send!(cmd: &[0x3A]); // mode 16 bit
    send!(data: &[0x06]);
    FreeRtos::delay_ms(200);
    send!(cmd: &[0x53]); //CTRL display
    send!(data: &[0b00100100]); //CTRL display
    FreeRtos::delay_ms(200);
    send!(cmd: &[0x36]); //CTRL display
    send!(data: &[0x00]); //CTRL display
    FreeRtos::delay_ms(200);
    send!(cmd: &[0x29]); //display on
    FreeRtos::delay_ms(200);
    send!(cmd: &[0x20]); //inversion off
    FreeRtos::delay_ms(200);
    send!(cmd: &[0x51]); //brightness
    send!(data: &[0xff]);
    FreeRtos::delay_ms(200);

    /*                                              dc.set_low().unwrap(); // command mode
        device.write(&[0x2A]).unwrap();
        dc.set_high().unwrap(); // data mode
        device.write(&[0x00, 0x0A, 0x00, 0x5A]).unwrap(); // X start = X end = 10
                                                          //
        dc.set_low().unwrap(); // command mode
        device.write(&[0x2B]).unwrap();
        dc.set_high().unwrap(); // data mode
        device.write(&[0x00, 0x0A, 0x00, 0x5A]).unwrap(); // X start = X end = 10
    */
    //
    /*dc.set_low().unwrap(); // command mode
    device.write(&[0x53]).unwrap();
    dc.set_high().unwrap(); // data mode
    device.write(&[0b00100100]).unwrap(); // X start = X end = 10*/
    println!("0");
    FreeRtos::delay_ms(400);
    send!(cmd: &[0x2a]);
    send!(data: &[0x00,0x01,0x00,0xef]);
    FreeRtos::delay_ms(400);
    send!(cmd: &[0x2b]);
    send!(data: &[0x00,0x01,0x00,0xef]);
    FreeRtos::delay_ms(400);
    send!(cmd: &[0x2c]);
    send!(cmd: &[0x3c]);
    send!(data: &[0xff;60]); //half screen???
    for _ in 1..4 {
        send!(data: &[0xff]);
        send!(data: &[0xff]);
        send!(data: &[0x00]);
        send!(data: &[0xff]);
        send!(data: &[0x00]);
        send!(data: &[0xff]);
        send!(data: &[0x00]);
        send!(data: &[0xff]);
        send!(data: &[0xff]);
    }
    send!(data: &[0x00;60]); //half screen???
    send!(data: &[0xff;60]); //half screen???
    send!(data: &[0x00;60]); //half screen???
    send!(data: &[0xff;60]); //half screen???
    send!(data: &[0x00;60]); //half screen???
    println!("1");
    FreeRtos::delay_ms(4000);
    send!(cmd: &[0x21]); //inversion off
    FreeRtos::delay_ms(2000);
    send!(cmd: &[0x20]); //inversion off
    FreeRtos::delay_ms(2000);
    send!(cmd: &[0x2c]);
    send!(data: &[0xff;240*240*2]);
    println!("done");
    /*
    loop {
        FreeRtos::delay_ms(800);
        send!(cmd: &[0x21]); //inversion on
        FreeRtos::delay_ms(4000);
        send!(cmd: &[0x20]); //inversion off
    } */

    //
}

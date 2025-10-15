#![allow(unused_imports)]
#![allow(unreachable_code)]
use esp_idf_hal::gpio::{AnyIOPin, InputPin, OutputPin};
use esp_idf_hal::ledc::config::TimerConfig;
use esp_idf_hal::ledc::{LedcDriver, LedcTimerDriver};
use esp_idf_hal::units::Time;
use esp_idf_svc::hal::uart::*;

use heapless::String;

use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::{gpio::PinDriver, peripherals::Peripherals};
use esp_idf_svc::io::EspIOError;

use esp_idf_svc::hal::ledc;

use log::*;
use std::io::{self, Read, Write};
use std::pin::Pin;

use std::net::TcpListener;

use std::sync::{Arc, Mutex};

use std::time::Instant;

mod servo;
use crate::servo::Servo;

mod motor;
use crate::motor::{Direction, Motor};

mod ydlidar;
use crate::ydlidar::ydlidar_gs2::{self, Ydlidar};

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    let gpio5 = PinDriver::input(peripherals.pins.gpio5)?;
    let gpio5 = Arc::new(Mutex::new(gpio5));

    //servo config
    let servo_timer_driver = LedcTimerDriver::new(
        peripherals.ledc.timer0,
        &TimerConfig::default()
            .frequency(50.into())
            .resolution(ledc::Resolution::Bits14),
    )?;
    let ledc1 = LedcDriver::new(
        peripherals.ledc.channel2,
        &servo_timer_driver,
        peripherals.pins.gpio3,
    )?;
    let ledc2 = LedcDriver::new(
        peripherals.ledc.channel1,
        &servo_timer_driver,
        peripherals.pins.gpio4,
    )?;

    let servo1 = Servo::new(ledc1, 90.0).unwrap();
    let servo2 = Servo::new(ledc2, 90.0).unwrap();

    info!("config servo fini");
    //motor config
    let pin_a1 = PinDriver::output(peripherals.pins.gpio10.downgrade_output())?;
    let motor_timer_driver = LedcTimerDriver::new(
        peripherals.ledc.timer1,
        &TimerConfig::default()
            .frequency(25_000.into())
            .resolution(ledc::Resolution::Bits10),
    )?;
    let pin_a2 = LedcDriver::new(
        peripherals.ledc.channel0,
        &motor_timer_driver,
        peripherals.pins.gpio9,
    )?;

    let motor_a = Motor::new(pin_a1, pin_a2);
    let pin_b1 = PinDriver::output(peripherals.pins.gpio21.downgrade_output())?;
    let pin_b2 = LedcDriver::new(
        peripherals.ledc.channel3,
        &motor_timer_driver,
        peripherals.pins.gpio20,
    )?;

    let motor_b = Motor::new(pin_b1, pin_b2);

    motor_a.set_dir(Direction::Forward)?;
    motor_b.set_dir(Direction::Forward)?;

    info!("config moteur fini");

    let uart = UartDriver::new(
        peripherals.uart1,
        peripherals.pins.gpio7,
        peripherals.pins.gpio6,
        None::<AnyIOPin>,
        None::<AnyIOPin>,
        &esp_idf_svc::hal::uart::config::Config::new()
            .baudrate(921600.into())
            .rx_fifo_size(256),
    )?;

    let start_time = Instant::now();
    info!("lidar config fini");
    let mut lidar = Ydlidar::new(uart, move || start_time.elapsed().as_millis() as u64);

    info!("try lidar_get_address");
    let lidar_address = lidar.get_address().unwrap();
    info!("lidar@ : 0x0{:x?}", lidar_address as u8);

    Ok(())
}

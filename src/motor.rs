use anyhow::*;
use esp_idf_hal::{gpio::Output, ledc::LedcDriver};
use esp_idf_svc::hal::gpio::{AnyOutputPin, PinDriver};
use std::sync::{Arc, Mutex};

#[derive(Clone, PartialEq, Debug)]
pub enum Direction {
    Forward,
    Backward,
    None,
}

#[derive(Clone)]
pub struct Motor<'a> {
    motor_pin_a: Arc<Mutex<PinDriver<'a, AnyOutputPin, Output>>>,
    motor_pin_b: Arc<Mutex<LedcDriver<'a>>>,
    speed: Arc<Mutex<f32>>,     // 0–100
    dir: Arc<Mutex<Direction>>, // sens
}

impl<'a> Motor<'a> {
    pub fn new(
        motor_pin_a: PinDriver<'a, AnyOutputPin, Output>,
        motor_pin_b: LedcDriver<'a>,
    ) -> Self {
        Motor {
            motor_pin_a: Arc::new(Mutex::new(motor_pin_a)),
            motor_pin_b: Arc::new(Mutex::new(motor_pin_b)),
            speed: Arc::new(Mutex::new(0.0)),
            dir: Arc::new(Mutex::new(Direction::None)),
        }
    }

    /// Fixe uniquement la direction
    pub fn set_dir(&self, dir: Direction) -> Result<()> {
        let mut pin = self.motor_pin_a.lock().unwrap();
        match dir {
            Direction::Forward => pin.set_high()?,
            Direction::Backward => pin.set_low()?,
            Direction::None => {
                pin.set_low()?;
                self.motor_pin_b.lock().unwrap().set_duty(0)?;
            }
        };
        drop(pin);
        *self.dir.lock().unwrap() = dir.clone();

        if dir != Direction::None {
            let speed = *self.speed.lock().unwrap();
            self.set_speed(speed)?;
        }
        Ok(())
    }

    /// Inverse la direction
    pub fn toggle_dir(&self) -> Result<()> {
        let dir = self.dir.lock().unwrap().clone();
        match dir {
            Direction::Forward => self.set_dir(Direction::Backward)?,
            Direction::Backward => self.set_dir(Direction::Forward)?,
            Direction::None => {}
        }
        Ok(())
    }

    /// Fixe la vitesse (0–100 %)
    pub fn set_speed(&self, speed: f32) -> Result<()> {
        let dir = self.dir.lock().unwrap().clone();
        if dir == Direction::None {
            *self.speed.lock().unwrap() = 0.0;
            return Ok(());
        }

        let mut ledc = self.motor_pin_b.lock().unwrap();
        let max_duty = ledc.get_max_duty();
        let speed = speed.clamp(0.0, 100.0);

        let mut duty = ((speed / 100.0) * max_duty as f32) as u32;
        if dir == Direction::Forward {
            duty = max_duty - duty;
        }
        ledc.set_duty(duty)?;
        *self.speed.lock().unwrap() = speed;
        Ok(())
    }

    pub fn set_power(&self, power: f32) -> Result<()> {
        if power == 0.0 {
            self.set_dir(Direction::None)?;
            *self.speed.lock().unwrap() = 0.0;
            return Ok(());
        }
        if power > 0.0 {
            self.set_dir(Direction::Forward)?;
            self.set_speed(power.abs())?;
        } else {
            self.set_dir(Direction::Backward)?;
            self.set_speed(power.abs())?;
        }
        Ok(())
    }

    /// Renvoie la vitesse signée
    pub fn get_speed(&self) -> f32 {
        match *self.dir.lock().unwrap() {
            Direction::None => 0.0,
            Direction::Forward => *self.speed.lock().unwrap(),
            Direction::Backward => -*self.speed.lock().unwrap(),
        }
    }
}

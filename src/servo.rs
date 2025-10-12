use anyhow::Result;
use esp_idf_hal::ledc::LedcDriver;

use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Servo<'a> {
    ledc_driver: Arc<Mutex<LedcDriver<'a>>>,
    current_value: Arc<Mutex<f32>>,
    max_duty: u32,
}

impl<'a> Servo<'a> {
    pub fn new(ledc_driver: LedcDriver<'a>, init_value: f32) -> Result<Self> {
        let max_duty = ledc_driver.get_max_duty();
        let servo = Servo {
            ledc_driver: Arc::new(Mutex::new(ledc_driver)),
            current_value: Arc::new(Mutex::new(init_value)),
            max_duty,
        };
        servo.set_angle(init_value)?;
        Ok(servo)
    }

    pub fn set_angle(&self, angle: f32) -> Result<()> {
        let mut driver = self.ledc_driver.lock().unwrap();
        let duty = self.calcul_duty(angle);
        driver.set_duty(duty)?;
        *self.current_value.lock().unwrap() = angle;
        Ok(())
    }

    pub fn get_angle(&self) -> f32 {
        *self.current_value.lock().unwrap()
    }

    fn calcul_duty(&self, angle: f32) -> u32 {
        let angle = angle.clamp(0.0, 180.0);
        let min_duty = (25 * self.max_duty) / 1000;
        let max_duty = (125 * self.max_duty) / 1000;
        let duty_gap = max_duty - min_duty;
        let duty = min_duty as f32 + ((angle * duty_gap as f32) / 180.0);
        duty as u32
    }
}

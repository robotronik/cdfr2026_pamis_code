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
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::http::server::EspHttpServer;
use esp_idf_svc::io::EspIOError;
use esp_idf_svc::ipv4;
use esp_idf_svc::nvs::{EspNvsPartition, NvsDefault};
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};

use esp_idf_svc::hal::ledc;

use embedded_svc::http::server::Method;

use serde_json::json;

use log::*;
use std::io::{self, Read, Write};
use std::pin::Pin;

use embedded_websocket::{
    framer::{Framer, ReadResult},
    WebSocketSendMessageType, WebSocketServer,
};
use std::net::TcpListener;

use std::sync::{Arc, Mutex};

mod servo;
use crate::servo::Servo;

mod motor;
use crate::motor::{Direction, Motor};

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let sys_loop = EspSystemEventLoop::take().unwrap();
    let nvs = <EspNvsPartition<NvsDefault>>::take().unwrap();

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

    const SSID: &str = env!("SSID");
    const PASSWORD: &str = env!("PASSWORD");
    log::info!("Connecting to Wi-Fi network '{SSID}'...");

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs)).unwrap(),
        sys_loop,
    )
    .unwrap();

    let ssid: String<32> = String::try_from(SSID).unwrap();
    let password: String<64> = String::try_from(PASSWORD).unwrap();

    let wifi_configuration =
        esp_idf_svc::wifi::Configuration::Client(esp_idf_svc::wifi::ClientConfiguration {
            ssid: ssid,
            password: password,
            auth_method: esp_idf_svc::wifi::AuthMethod::WPAWPA2Personal,
            ..Default::default()
        });

    wifi.set_configuration(&wifi_configuration).unwrap();
    wifi.start()?;
    let scan_results = wifi.scan()?;
    log::info!("Found {} Wi-Fi networks:", scan_results.len());

    let mut found = false;
    for ap in &scan_results {
        let ssid_str = ap.ssid.as_str(); // pas besoin de from_utf8
        log::info!("  SSID: {}, Signal: {}", ssid_str, ap.signal_strength);
        if ssid_str == SSID {
            found = true;
        }
    }
    if !found {
        log::warn!("⚠️ SSID '{}' not found during scan!", SSID);
        // ici tu peux décider de retenter un scan, attendre, ou carrément redémarrer l'ESP32
    }
    connect_wifi(&mut wifi)?;

    let ip_info: ipv4::IpInfo = wifi.wifi().sta_netif().get_ip_info().unwrap();
    log::info!("Wi-Fi connected! IP address: {}", ip_info.ip);
    let mut server = EspHttpServer::new(&Default::default()).unwrap();

    server
        .fn_handler("/", Method::Get, |request| -> Result<(), EspIOError> {
            log::info!("Received request");
            let mut response = request.into_ok_response()?;
            response.write(index_html().as_bytes())?;
            Ok(())
        })
        .unwrap();

    log::info!(
        "Server is running. Open http://{} in your browser",
        ip_info.ip
    );

    let listener = TcpListener::bind("0.0.0.0:8080")?;
    info!("WebSocket server listening on ws://{}:8080", ip_info.ip);

    for stream in listener.incoming() {
        let mut stream = stream?;
        let servo1_clone = servo1.clone();
        let servo2_clone = servo2.clone();
        let motor_a_clone = motor_a.clone();
        let motor_b_clone = motor_b.clone();
        let gpio5_clone = gpio5.clone();

        std::thread::Builder::new()
            .stack_size(8192)
            .spawn(move || -> anyhow::Result<()> {
                let mut websocket = WebSocketServer::new_server();

                let mut read_buf = Box::new([0u8; 1024]);
                let mut write_buf = Box::new([0u8; 1024]);
                let mut read_cursor = 0;

                let n = stream.read(&mut *read_buf).unwrap();
                let mut headers_arr = [httparse::EMPTY_HEADER; 16];
                let mut request = httparse::Request::new(&mut headers_arr);
                request.parse(&read_buf[..n]).unwrap();

                let header_iter = request.headers.iter().map(|h| (h.name, h.value));

                let websocket_context = match embedded_websocket::read_http_header(header_iter)? {
                    Some(ctx) => ctx,
                    None => {
                        log::warn!("Client did not request WebSocket upgrade, closing connection");
                        return Ok(());
                    }
                };
                let size = websocket.server_accept(
                    &websocket_context.sec_websocket_key,
                    None,
                    &mut *write_buf,
                )?;
                stream.write_all(&write_buf[..size]).unwrap();

                // 2️⃣ Créer le framer
                let mut framer = Framer::new(
                    &mut *read_buf,
                    &mut read_cursor,
                    &mut *write_buf,
                    &mut websocket,
                );

                stream.set_nonblocking(true).unwrap();
                let mut last_status = std::time::Instant::now();

                let mut old_angle = servo1_clone.get_angle();
                //for send the status the first time
                let mut old_gpio5_state = !gpio5_clone.lock().unwrap().is_high() as u8;

                loop {
                    let mut temp_buf = [0u8; 512];
                    match framer.read(&mut stream, &mut temp_buf) {
                        Ok(ReadResult::Text(text)) => {
                            //  info!("Received: {}", text);
                            if let Some(angle_str) = text.strip_prefix("servo1=") {
                                if let Ok(angle) = angle_str.parse::<u32>() {
                                    servo1_clone.set_angle(angle as f32).unwrap();
                                }
                            }
                            if let Some(angle_str) = text.strip_prefix("servo2=") {
                                if let Ok(angle) = angle_str.parse::<u32>() {
                                    servo2_clone.set_angle(angle as f32).unwrap();
                                }
                            }
                            if let Some(speed_str) = text.strip_prefix("moteurAspeed=") {
                                if let Ok(speed) = speed_str.parse::<u32>() {
                                    motor_a_clone.set_speed(speed as f32).unwrap();
                                }
                            }
                            if let Some(speed_str) = text.strip_prefix("moteurBspeed=") {
                                if let Ok(speed) = speed_str.parse::<u32>() {
                                    motor_b_clone.set_speed(speed as f32).unwrap();
                                }
                            }
                            if let Some(val) = text.strip_prefix("motor1=") {
                                if let Ok(speed) = val.parse::<f32>() {
                                    info!("Received : motor1={speed}");
                                    if speed >= 0.0 {
                                        motor_a_clone.set_dir(Direction::Forward)?;
                                    } else {
                                        motor_a_clone.set_dir(Direction::Backward)?;
                                    }
                                    motor_a_clone.set_speed(speed.abs())?;
                                }
                            }

                            if let Some(val) = text.strip_prefix("motor2=") {
                                if let Ok(speed) = val.parse::<f32>() {
                                    info!("Received : motor2={speed}");
                                    if speed >= 0.0 {
                                        motor_b_clone.set_dir(Direction::Forward)?;
                                    } else {
                                        motor_b_clone.set_dir(Direction::Backward)?;
                                    }
                                    motor_b_clone.set_speed(speed.abs())?;
                                }
                            }
                        }
                        Ok(ReadResult::Closed) => {
                            info!("Client disconnected");
                            break Ok(());
                        }
                        Err(embedded_websocket::framer::FramerError::Io(e))
                            if e.kind() == std::io::ErrorKind::WouldBlock =>
                        {
                            // Pas de message dispo
                        }
                        Err(embedded_websocket::framer::FramerError::WebSocket(
                            embedded_websocket::Error::ReadFrameIncomplete,
                        )) => {
                            //un petit soucis, mais rien de grave -> ferme pas la connection
                            warn!("Websocket error ignored : ReadFrameIncomplete");
                            continue;
                        }
                        Err(e) => {
                            log::error!("WebSocket error: {:?}", e);
                            break Ok(());
                        }
                        _ => {}
                    }

                    if last_status.elapsed() >= std::time::Duration::from_millis(100) {
                        let angle = servo1_clone.get_angle();
                        let gpio5_state = gpio5_clone.lock().unwrap().is_high() as u8;
                        if (angle != old_angle) || (gpio5_state != old_gpio5_state) {
                            let status = json!({
                                "angle": angle,
                                "gpio5": gpio5_state
                            });
                            framer
                                .write(
                                    &mut stream,
                                    WebSocketSendMessageType::Text,
                                    true,
                                    status.to_string().as_bytes(),
                                )
                                .unwrap();
                            last_status = std::time::Instant::now();
                            info!("status {:?} send at {:?}", status, last_status);
                            old_angle = angle;
                            old_gpio5_state = gpio5_state;
                        }
                    }
                    FreeRtos::delay_ms(10);
                }
            })?;
    }

    loop {
        //never
        info!("loop !! ");

        FreeRtos::delay_ms(1000);
    }
    Ok(())
}
fn index_html() -> &'static str {
    include_str!("static/index.html")
}

use std::time::{Duration, Instant};

fn connect_wifi(wifi: &mut BlockingWifi<EspWifi<'static>>) -> anyhow::Result<()> {
    let mut attempt = 0;
    loop {
        attempt += 1;
        log::info!("Connecting to Wi-Fi... attempt {}", attempt);

        if let Err(e) = wifi.connect() {
            log::warn!("Wi-Fi connect error: {:?}", e);
        }

        // Timeout d'attente de 10 secondes max
        let start = Instant::now();
        loop {
            match wifi.wait_netif_up() {
                Ok(_) => {
                    log::info!("Wi-Fi connected!");
                    return Ok(());
                }
                Err(e) => {
                    if start.elapsed() > Duration::from_secs(10) {
                        log::warn!("Timeout waiting for netif_up: {:?}", e);
                        break; // on sort pour réessayer
                    }
                    FreeRtos::delay_ms(500);
                }
            }
        }
    }
}

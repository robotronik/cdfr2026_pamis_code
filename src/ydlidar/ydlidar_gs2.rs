use anyhow::Result;
use esp_idf_svc::hal::io::{Read, Write};

/* Protocol Format:
+-------------+---------+-------------+-------------------+--------------+--------------------+
|   HEADER    | ADDRESS | PACKET TYPE |   PACKET LENGTH   |    PACKET    |     CHECK CODE     |
+-------------+---------+-------------+-------------------+--------------+--------------------+
| A5 A5 A5 A5 |   0x0*  |     0x**    |(LSB)0x** (MSB)0x**|(Length)*0x** | (packet type=)0x** |
+-------------+---------+-------------+-------------------+--------------+--------------------+
*/

use crate::ydlidar::ydlidar_cmd::{self, YdlidarAddress, YdlidarCmd, YdlidarProtocol};

pub struct Ydlidar<UART> {
    uart: UART,
    address: YdlidarAddress,
    now_ms: Box<dyn Fn() -> u64>,
}

#[derive(Debug)]
pub enum YdlidarError<e> {
    Uart(e),
    Timeout,
    ChecksumError,
}

impl<UART> Ydlidar<UART>
where
    UART: Read + Write,
{
    pub fn new(mut uart: UART, now_ms: impl Fn() -> u64 + 'static) -> Self {
        uart.flush().unwrap();
        Self {
            uart,
            address: YdlidarAddress::Broadcast,
            now_ms: Box::new(now_ms),
        }
    }

    pub fn config_address(&mut self, address: YdlidarAddress) {
        self.address = address;
    }

    pub fn send_uart(&mut self, cmd: u8, payload: &[u8]) -> Result<(), UART::Error> {
        let mut checksum: u8 = cmd;

        self.uart.write(&[cmd])?;

        for &b in payload {
            checksum = checksum.wrapping_add(b);
            self.uart.write(&[b])?;
        }

        self.uart.write(&[checksum])?;
        Ok(())
    }

    pub fn get_address(&mut self) -> Result<YdlidarAddress, YdlidarError<UART::Error>> {
        let address = YdlidarAddress::Broadcast;
        self.address = address;
        log::info!("send_command");
        if let Err(e) = self.send_command(YdlidarCmd::GetAddress) {
            return Err(YdlidarError::Uart(e));
        };
        let mut buf: [u8; 9] = [0; 9];
        log::info!("read_command");
        // self.read_exact_with_timeout(&mut buf, 300)?;

        self.uart.read(&mut buf).unwrap();
        log::info!("lidar response : {:?}", &buf);

        let checksum: u8 = buf[4..buf.len() - 1]
            .iter()
            .fold(0u8, |acc, &b| acc.wrapping_add(b));
        if checksum != buf[8] {
            return Err(YdlidarError::ChecksumError);
        }
        let address = YdlidarAddress::try_from(buf[4]).unwrap();
        self.address = address;
        Ok(address)
    }

    pub fn read_exact_with_timeout(
        &mut self,
        buf: &mut [u8],
        timeout_ms: u64,
    ) -> Result<(), YdlidarError<UART::Error>> {
        let mut filled = 0;
        let now_ms = self.now_ms.as_ref();
        let start = now_ms();

        while filled < buf.len() {
            match self.uart.read(&mut buf[filled..]) {
                Ok(0) => {
                    // rien reçu → check timeout
                    if now_ms().saturating_sub(start) > timeout_ms {
                        return Err(YdlidarError::Timeout);
                    }
                }
                Ok(n) => filled += n,
                Err(e) => return Err(YdlidarError::Uart(e)),
            }
        }

        Ok(())
    }

    pub fn read_response(
        &mut self,
        buffer: &mut [u8],
    ) -> Result<(), embedded_io::ReadExactError<UART::Error>> {
        self.uart.read_exact(buffer)?;
        Ok(())
    }

    pub fn send_command(&mut self, cmd: YdlidarCmd) -> Result<(), UART::Error> {
        let protocol = cmd.to_protocol(self.address.clone(), None);
        let buf = protocol.to_bytes();
        log::info!("send {:X?}", &buf);
        self.uart.write_all(&buf)?;
        Ok(())
    }

    pub fn send_command_packet(
        &mut self,
        cmd: YdlidarCmd,
        packet: &[u8],
    ) -> Result<(), UART::Error> {
        let protocol = cmd.to_protocol(self.address.clone(), Some(packet));
        let buf = protocol.to_bytes();
        self.uart.write_all(&buf)?;
        Ok(())
    }
}

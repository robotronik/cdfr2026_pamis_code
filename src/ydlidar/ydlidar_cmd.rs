use anyhow::Result;
use esp_idf_hal::i2c::Operation;

pub enum YdlidarCmd {
    SyncByte,
    GetAddress,
    GetParameter,
    GetVersion,
    Scan,
    Stop,
    Reset,
    SetBias,
    SetDebugMode,
}

impl YdlidarCmd {
    pub const fn as_bytes(&self) -> &'static [u8] {
        match self {
            Self::SyncByte => &[0xA5],
            Self::GetAddress => &[0x60],
            Self::GetParameter => &[0x61],
            Self::GetVersion => &[0x62],
            Self::Scan => &[0x63],
            Self::Stop => &[0x64],
            Self::Reset => &[0x67],
            Self::SetBias => &[0xD9],
            Self::SetDebugMode => &[0xF0],
        }
    }

    pub fn to_protocol(
        self,
        address: YdlidarAddress,
        packet: Option<&[u8]>,
    ) -> YdlidarProtocol<'_> {
        YdlidarProtocol::new(address, self, packet)
    }
}
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum YdlidarAddress {
    Broadcast = 0x00,
    Device1 = 0x01,
    Device2 = 0x02,
    Device3 = 0x04,
}

impl TryFrom<u8> for YdlidarAddress {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(YdlidarAddress::Broadcast),
            0x01 => Ok(YdlidarAddress::Device1),
            0x02 => Ok(YdlidarAddress::Device2),
            0x04 => Ok(YdlidarAddress::Device3),
            _ => Err(()),
        }
    }
}

/* Protocol Format:
+-------------+---------+-------------+-------------------+--------------+--------------------+
|   HEADER    | ADDRESS | PACKET TYPE |   PACKET LENGTH   |    PACKET    |     CHECK CODE     |
+-------------+---------+-------------+-------------------+--------------+--------------------+
| A5 A5 A5 A5 |   0x0*  |     0x**    |(LSB)0x** (MSB)0x**|(Length)*0x** | (packet type=)0x** |
+-------------+---------+-------------+-------------------+--------------+--------------------+
*/

pub struct YdlidarProtocol<'a> {
    address: YdlidarAddress,
    cmd: YdlidarCmd,
    size: u16,
    packet: &'a [u8],
}

impl<'a> YdlidarProtocol<'a> {
    pub fn new(address: YdlidarAddress, cmd: YdlidarCmd, payload: Option<&'a [u8]>) -> Self {
        YdlidarProtocol {
            address,
            cmd,
            size: match payload {
                Some(a) => a.len() as u16,
                None => 0,
            },
            packet: payload.unwrap_or(&[]),
        }
    }

    pub fn to_bytes(self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(45);
        //sync byte
        buf.push(0xA5);
        buf.push(0xA5);
        buf.push(0xA5);
        buf.push(0xA5);
        //address
        buf.push(self.address as u8);
        //command
        buf.extend_from_slice(self.cmd.as_bytes());
        //size
        buf.push((self.size & 0xFF) as u8); //LSB
        buf.push((self.size >> 8) as u8); //MSB
                                          //packet
        buf.extend_from_slice(self.packet);
        // checksum (sur tout ce qui précède)
        let checksum: u8 = buf[4..].iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        buf.push(checksum);
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_as_bytes() {
        assert!(YdlidarCmd::SyncByte.as_bytes(), &[0xA5]);
        assert!(YdlidarCmd::GetAddress.as_bytes(), &[0x60]);
        assert!(YdlidarCmd::GetParameter.as_bytes(), &[0x61]);
        assert!(YdlidarCmd::GetVersion.as_bytes(), &[0x62]);
        assert!(YdlidarCmd::Scan.as_bytes(), &[0x63]);
        assert!(YdlidarCmd::Stop.as_bytes(), &[0x64]);
        assert!(YdlidarCmd::Reset.as_bytes(), &[0x67]);
        assert!(YdlidarCmd::SetBias.as_bytes(), &[0xD9]);
        assert!(YdlidarCmd::SetDebugMode.as_bytes(), &[0xF0]);
    }

    #[test]
    fn test_protocol_as_bytes() {
        assert_eq!(
            YdlidarProtocol::new(YdlidarAddress::Broadcast, YdlidarCmd::GetAddress, None)
                .to_bytes(),
            &[0xA5, 0xA5, 0xA5, 0xA5, 0x00, 0x60, 0x00, 0x00, 0x60]
        );
    }
}

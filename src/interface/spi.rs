

use embedded_hal;
// use embedded_hal::{
//     digital::v2::OutputPin,
// };

use super::{SensorInterface};
use crate::interface::{PACKET_HEADER_LENGTH, SensorCommon};
use embedded_hal::digital::v2::{OutputPin, InputPin};
use crate::Error;


/// This combines the SPI peripheral and a data/command pin
pub struct SpiInterface<SPI, CS, IN, WN> {
    spi: SPI,
    cs: CS,
    hintn: IN,
    waken: WN,
    received_packet_count: usize,
}

impl<SPI, CS, IN, WN, CommE, PinE> SpiInterface<SPI, CS, IN, WN>
    where
        SPI: embedded_hal::blocking::spi::Write<u8, Error = CommE> +
        embedded_hal::blocking::spi::Transfer<u8, Error = CommE>,
        CS: OutputPin<Error = PinE>,
        IN: InputPin<Error = PinE>,
        WN: OutputPin<Error = PinE>
{
    pub fn new(spi: SPI, cs: CS, hintn: IN, waken: WN) -> Self {
        Self {
            spi,
            cs,
            hintn,
            waken,
            received_packet_count: 0
        }
    }

    fn data_available(&self) ->  bool  {
        self.hintn.is_low().unwrap_or(false)
    }
}

impl<SPI, CS, IN, WN, CommE, PinE> SensorInterface for SpiInterface<SPI, CS, IN, WN>
    where
        SPI: embedded_hal::blocking::spi::Write<u8, Error = CommE> +
        embedded_hal::blocking::spi::Transfer<u8, Error = CommE>,
        CS: OutputPin<Error = PinE>,
        IN: InputPin<Error = PinE>,
        WN: OutputPin<Error = PinE>
{
    type SensorError = Error<CommE, PinE>;

    fn setup(&mut self) -> Result<(), Self::SensorError> {
        self.waken.set_high().map_err(Error::Pin)?;
        Ok(())
    }

    fn send_packet(&mut self, packet: &[u8]) -> Result<(), Self::SensorError> {
        //self.waken.set_low().map_err(Error::Pin)?;
        self.cs.set_low().map_err(Error::Pin)?;

        self.spi.write(&packet).map_err(Error::Comm)?;
        self.cs.set_high().map_err(Error::Pin)?;
        Ok(())
    }

    fn read_packet(&mut self, recv_buf: &mut [u8]) -> Result<usize, Self::SensorError> {
        //self.waken.set_low().map_err(Error::Pin)?;

        if !self.data_available() {
            return Ok(0)
        }

        self.cs.set_low().map_err(Error::Pin)?;

        //ensure that buffer is zeroed since we're not sending any data
        for i in recv_buf.iter_mut() {
            *i = 0;
        }
        //TODO might need to look at INTN pin to detect whether a packet is available
        // get just the header
        self.spi.transfer(&mut recv_buf[..PACKET_HEADER_LENGTH]).map_err(Error::Comm)?;
        let mut packet_len = SensorCommon::parse_packet_header(&recv_buf[..PACKET_HEADER_LENGTH]);
        if packet_len > 300 {
            // equivalent of 0xFFFF is garbage
            packet_len = 0;
        }
        if packet_len > PACKET_HEADER_LENGTH {
            self.spi.transfer( &mut recv_buf[PACKET_HEADER_LENGTH..packet_len]).map_err(Error::Comm)?;
        }

        if  packet_len > 0 {
            self.received_packet_count += 1;
        }

        self.cs.set_high().map_err(Error::Pin)?;
        Ok(packet_len)
    }
}

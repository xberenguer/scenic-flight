//! CO5300 SPI2 transfers. The game framebuffer remains independent of wire order.

use esp_hal::{
    Blocking,
    delay::Delay,
    dma::DmaTxBuf,
    gpio::Output,
    spi::{
        Error,
        master::{Address, Command, DataMode, SpiDma},
    },
};

use crate::bands::{BANDS, BUFFER_BYTES, Band, Scan, Scanline};
#[cfg(feature = "native-flight-overlay")]
use crate::native_flight::NativeFlight;
#[cfg(feature = "native-airframe-probe")]
use crate::packed_airframe::PackedCapture;

pub struct Panel<const LOGICAL: usize> {
    spi: Option<SpiDma<'static, Blocking>>,
    buffers: [Option<DmaTxBuf>; 2],
    scan: Scan<LOGICAL>,
    line: Scanline,
    visible: bool,
}

impl<const LOGICAL: usize> Panel<LOGICAL> {
    pub fn new(
        spi: SpiDma<'static, Blocking>,
        buffers: [DmaTxBuf; 2],
        reset: &mut Output<'static>,
    ) -> Result<Self, Error> {
        let mut panel = Self {
            spi: Some(spi),
            buffers: buffers.map(Some),
            scan: Scan::new(),
            line: Scanline::new(),
            visible: false,
        };
        let delay = Delay::new();
        reset.set_high();
        delay.delay_millis(10);
        reset.set_low();
        delay.delay_millis(200);
        reset.set_high();
        delay.delay_millis(200);
        panel.register(0x11, &[])?;
        delay.delay_millis(120);
        for (register, value) in [
            (0xfe, 0),
            (0xc4, 0x80),
            (0x3a, 0x55),
            (0x53, 0x20),
            (0x63, 0xff),
            (0x51, 0),
            (0x58, 0),
            (0x36, 0),
        ] {
            panel.register(register, &[value])?;
        }
        panel.register(0x20, &[])?;
        panel.register(0x28, &[])?;
        Ok(panel)
    }

    fn register(&mut self, register: u8, data: &[u8]) -> Result<(), Error> {
        self.spi.as_mut().unwrap().half_duplex_write(
            DataMode::SingleTwoDataLines,
            Command::_8Bit(0x02, DataMode::SingleTwoDataLines),
            Address::_24Bit(u32::from(register) << 8, DataMode::SingleTwoDataLines),
            0,
            data,
        )
    }

    fn window(&mut self, band: Band) -> Result<(), Error> {
        let (columns, rows) = band.window();
        self.register(0x2a, &columns)?;
        self.register(0x2b, &rows)?;
        self.register(0x2c, &[])
    }

    fn pack(
        &mut self, image: &[u16], slot: usize, band: Band,
        #[cfg(feature = "native-flight-overlay")] native: &NativeFlight<'_>,
        #[cfg(feature = "native-airframe-probe")] capture: Option<&mut PackedCapture<'_>>,
    ) {
        let buffer = self.buffers[slot].as_mut().unwrap();
        buffer.set_length(band.bytes());
        self.scan.pack(image, buffer.as_mut_slice(), band, &mut self.line);
        #[cfg(feature = "native-flight-overlay")]
        native.pack(&self.scan, buffer.as_mut_slice(), band);
        #[cfg(feature = "native-airframe-probe")]
        if let Some(capture) = capture {
            capture.record_band(buffer.as_mut_slice(), band);
        }
    }

    pub fn present(
        &mut self, image: &[u16],
        #[cfg(feature = "native-flight-overlay")] native: &NativeFlight<'_>,
        #[cfg(feature = "native-airframe-probe")] mut capture: Option<&mut PackedCapture<'_>>,
        mut poll: impl FnMut(),
    ) -> Result<u32, Error> {
        assert_eq!(image.len(), LOGICAL * LOGICAL);
        let mut bytes = 0;
        self.line.begin_frame();
        self.pack(
            image, 0, self.scan.band(0),
            #[cfg(feature = "native-flight-overlay")] native,
            #[cfg(feature = "native-airframe-probe")] capture.as_deref_mut(),
        );
        for index in 0..BANDS {
            let slot = index % self.buffers.len();
            let band = self.scan.band(index);
            self.window(band)?;
            let spi = self.spi.take().unwrap();
            let buffer = self.buffers[slot].take().unwrap();
            let transfer = spi.half_duplex_write_buffer(
                DataMode::Quad,
                Command::_8Bit(0x32, DataMode::SingleTwoDataLines),
                Address::_24Bit(0x003c00, DataMode::SingleTwoDataLines),
                0,
                band.bytes(),
                buffer,
            );
            let transfer = match transfer {
                Ok(transfer) => transfer,
                Err((error, spi, buffer)) => {
                    self.spi = Some(spi);
                    self.buffers[slot] = Some(buffer);
                    return Err(error);
                }
            };
            // The transfer owns its buffer. Only the alternate buffer is filled here.
            if index + 1 < BANDS {
                self.pack(
                    image, 1 - slot, self.scan.band(index + 1),
                    #[cfg(feature = "native-flight-overlay")] native,
                    #[cfg(feature = "native-airframe-probe")] capture.as_deref_mut(),
                );
            }
            while !transfer.is_done() {
                poll();
            }
            let (spi, buffer) = transfer.wait();
            self.spi = Some(spi);
            self.buffers[slot] = Some(buffer);
            bytes += band.bytes() as u32;
            poll();
        }

        if !self.visible {
            self.register(0x29, &[])?;
            self.register(0x51, &[160])?;
            self.visible = true;
        }
        Ok(bytes)
    }
}

pub fn pixel_buffers() -> [DmaTxBuf; 2] {
    [
        esp_hal::dma_tx_buffer!(BUFFER_BYTES).unwrap(),
        esp_hal::dma_tx_buffer!(BUFFER_BYTES).unwrap(),
    ]
}

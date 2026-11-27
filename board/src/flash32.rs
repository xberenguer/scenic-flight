//! Read high physical addresses on the Waveshare's 32 MiB GD25Q256 flash.

use core::ptr;
use esp_hal::{
    peripherals::{CPU_CTRL, SPI1},
    system::{Cpu, CpuControl, is_running},
};

const FLASH_BYTES: u32 = 32 * 1024 * 1024;
const FAST_READ_4B: u32 = 0x0c;
const SINGLE_LANE_FAST_READ: i32 = 4;
const FLASH_CHIP_SELECT: u32 = 1;
const FIFO_BYTES: usize = 64;
const BURST_BYTES: usize = 512;

#[repr(align(4))]
struct InternalBuffer([u8; BURST_BYTES]);

unsafe extern "C" {
    // ESP32-S3 ROM ABI from esp32s3/rom/opi_flash.h. The mode selects
    // the electrical protocol, even on quad flash that is in SPI mode.
    fn esp_rom_opiflash_exec_cmd(
        spi: i32,
        mode: i32,
        command: u32,
        command_bits: i32,
        address: u32,
        address_bits: i32,
        dummy_clocks: i32,
        tx: *mut u8,
        tx_bits: i32,
        rx: *mut u8,
        rx_bits: i32,
        chip_select: u32,
        write_or_erase: bool,
    );
    #[link_name = "rom_Cache_Suspend_ICache"]
    fn suspend_instruction_cache() -> u32;
    #[link_name = "rom_Cache_Suspend_DCache"]
    fn suspend_data_cache() -> u32;
    #[link_name = "Cache_Resume_ICache"]
    fn resume_instruction_cache(autoload: u32);
    #[link_name = "Cache_Resume_DCache"]
    fn resume_data_cache(autoload: u32);
}

/// Read only. The board runs this on the main CPU with an internal stack,
/// and transfers to the destination after restoring the external caches.
pub fn read(mut address: u32, bytes: &mut [u8]) -> bool {
    if address > FLASH_BYTES || bytes.len() > (FLASH_BYTES - address) as usize {
        return false;
    }
    let mut scratch = InternalBuffer([0; BURST_BYTES]);
    for output in bytes.chunks_mut(BURST_BYTES) {
        critical_section::with(|_| {
            // Match the core parking used by esp-storage with AutoPark.
            let mut control = CpuControl::new(unsafe { CPU_CTRL::steal() });
            let active = Cpu::other().find(|&core| is_running(core));
            if let Some(core) = active {
                unsafe { control.park_core(core) };
            }
            // The flash command, register restoration, and scratch updates
            // use only ROM, IRAM, registers, and the internal CPU stack.
            unsafe { read_burst(address, scratch.0.as_mut_ptr(), output.len()) };
            if let Some(core) = active {
                control.unpark_core(core);
            }
        });
        output.copy_from_slice(&scratch.0[..output.len()]);
        address += output.len() as u32;
    }
    true
}

/// The caller excludes the other CPU and interrupts and supplies an aligned
/// writable internal-memory buffer of at least count bytes.
#[esp_hal::ram]
unsafe fn read_burst(mut address: u32, mut output: *mut u8, mut count: usize) {
    let spi = SPI1::regs();
    let user = spi.user().read().bits();
    let user1 = spi.user1().read().bits();
    let user2 = spi.user2().read().bits();
    let ctrl = spi.ctrl().read().bits();
    let ctrl1 = spi.ctrl1().read().bits();
    let ctrl2 = spi.ctrl2().read().bits();
    let clock = spi.clock().read().bits();
    let addr = spi.addr().read().bits();
    let misc = spi.misc().read().bits();
    let mosi = spi.mosi_dlen().read().bits();
    let miso = spi.miso_dlen().read().bits();
    let ddr = spi.ddr().read().bits();
    unsafe {
        let instruction_autoload = suspend_instruction_cache();
        let data_autoload = suspend_data_cache();
        while count != 0 {
            let packet = count.min(FIFO_BYTES);
            esp_rom_opiflash_exec_cmd(
                1,
                SINGLE_LANE_FAST_READ,
                FAST_READ_4B,
                8,
                address,
                32,
                8,
                ptr::null_mut(),
                0,
                output,
                (packet * 8) as i32,
                FLASH_CHIP_SELECT,
                false,
            );
            address += packet as u32;
            output = output.add(packet);
            count -= packet;
        }
        // Restore configuration before either core next uses external RAM
        // or the legacy flash reader. FIFO data and command strobes are transient.
        spi.user().write(|w| w.bits(user));
        spi.user1().write(|w| w.bits(user1));
        spi.user2().write(|w| w.bits(user2));
        spi.ctrl().write(|w| w.bits(ctrl));
        spi.ctrl1().write(|w| w.bits(ctrl1));
        spi.ctrl2().write(|w| w.bits(ctrl2));
        spi.clock().write(|w| w.bits(clock));
        spi.addr().write(|w| w.bits(addr));
        spi.misc().write(|w| w.bits(misc));
        spi.mosi_dlen().write(|w| w.bits(mosi));
        spi.miso_dlen().write(|w| w.bits(miso));
        spi.ddr().write(|w| w.bits(ddr));
        resume_data_cache(data_autoload);
        resume_instruction_cache(instruction_autoload);
    }
}

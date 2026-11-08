//! Flash algorithm for the TI CC26x0 (CC2640, CC2650): 128 KB of flash in 4 KB sectors, the
//! last one holding the CCFG.
//!
//! Erase and program go through the flash functions of the boot ROM, found from the ROM API
//! table, as TI's driverlib and OpenOCD's cc26xx loader do.

#![no_std]
#![no_main]

use flash_algorithm::*;

struct Algorithm {
    /// VIMS:CTL before the flash cache was turned off.
    vims_ctl: u32,
}

algorithm!(Algorithm, {
    device_name: "cc26x0",
    device_type: DeviceType::Onchip,
    flash_address: 0x0,
    flash_size: 0x20000,
    page_size: 0x1000,
    empty_value: 0xFF,
    program_time_out: 1000,
    erase_time_out: 2000,
    sectors: [{
        size: 0x1000,
        address: 0x0,
    }]
});

const FLASH_SIZE: u32 = 0x20000;
const SECTOR_SIZE: u32 = 0x1000;

/// The ROM API table; entry 10 points to the flash functions.
const ROM_API_TABLE: *const u32 = 0x1000_0180 as *const u32;
const ROM_FLASH_SECTOR_ERASE: usize = 5;
const ROM_FLASH_PROGRAM: usize = 6;

const VIMS_STAT: *mut u32 = 0x4003_4000 as *mut u32;
const VIMS_CTL: *mut u32 = 0x4003_4004 as *mut u32;
const VIMS_STAT_MODE_CHANGING: u32 = 1 << 3;
/// Cache off, line buffers disabled
const VIMS_CTL_OFF: u32 = 0x33;

/// FLASH:CFG, bit DIS_STANDBY, through the bit-band alias. The ROM functions may set it.
const FLASH_CFG_DIS_STANDBY: *mut u32 = 0x4260_0484 as *mut u32;

/// The security part of the CCFG (BL_CONFIG to CCFG_TAP_DAP_1): no boot loader backdoor,
/// TI failure analysis, the CPU DAP and the TAPs enabled.
const CCFG_SECURITY: u32 = FLASH_SIZE - SECTOR_SIZE + 0xFD8;
const CCFG_SECURITY_DEFAULT: [u8; 20] = [
    0xFF, 0xFF, 0xFF, 0xC5, 0xFF, 0xFF, 0xFF, 0xFF, 0xC5, 0xFF, 0xFF, 0xFF, 0xC5, 0xC5, 0xC5,
    0xFF, 0xC5, 0xC5, 0xC5, 0xFF,
];

fn rom_flash_function(index: usize) -> usize {
    unsafe {
        let table = ROM_API_TABLE.add(10).read_volatile() as *const u32;
        table.add(index).read_volatile() as usize
    }
}

fn status(status: u32) -> Result<(), ErrorCode> {
    unsafe { FLASH_CFG_DIS_STANDBY.write_volatile(0) };
    match ErrorCode::new(status) {
        None => Ok(()),
        Some(error) => Err(error),
    }
}

fn wait_vims() {
    while unsafe { VIMS_STAT.read_volatile() } & VIMS_STAT_MODE_CHANGING != 0 {}
}

impl FlashAlgorithm for Algorithm {
    fn new(_address: u32, _clock: u32, _function: Function) -> Result<Self, ErrorCode> {
        cortex_m::interrupt::disable();

        wait_vims();
        let vims_ctl = unsafe { VIMS_CTL.read_volatile() };
        unsafe { VIMS_CTL.write_volatile(vims_ctl | VIMS_CTL_OFF) };
        wait_vims();

        Ok(Self { vims_ctl })
    }

    fn erase_all(&mut self) -> Result<(), ErrorCode> {
        for address in (0..FLASH_SIZE).step_by(SECTOR_SIZE as usize) {
            self.erase_sector(address)?;
        }
        // An erased CCFG disables the debug access at the next reset.
        self.program_page(CCFG_SECURITY, &CCFG_SECURITY_DEFAULT)
    }

    fn erase_sector(&mut self, address: u32) -> Result<(), ErrorCode> {
        let erase: extern "C" fn(u32) -> u32 =
            unsafe { core::mem::transmute(rom_flash_function(ROM_FLASH_SECTOR_ERASE)) };
        status(erase(address))
    }

    fn program_page(&mut self, address: u32, data: &[u8]) -> Result<(), ErrorCode> {
        let program: extern "C" fn(*const u8, u32, u32) -> u32 =
            unsafe { core::mem::transmute(rom_flash_function(ROM_FLASH_PROGRAM)) };
        status(program(data.as_ptr(), address, data.len() as u32))
    }
}

impl Drop for Algorithm {
    fn drop(&mut self) {
        unsafe { VIMS_CTL.write_volatile(self.vims_ctl) };
        wait_vims();
        unsafe { cortex_m::interrupt::enable() };
    }
}

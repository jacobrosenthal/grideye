// ------------------------------------------------------------------------------
// Copyright 2018 Uwe Arzt, mail@uwe-arzt.de
// SPDX-License-Identifier: Apache-2.0
// ------------------------------------------------------------------------------

#![no_main]
#![no_std]

use grideye::{Address, GridEye, Power};
use panic_halt as _;
use stm32f4xx_hal::{delay::Delay, i2c::I2c, prelude::*, stm32 as hal};

#[cortex_m_rt::entry]
fn main() -> ! {
    let dp = hal::Peripherals::take().unwrap();
    let cp = cortex_m::peripheral::Peripherals::take().unwrap();

    // Set up the system clock. We want to run at 48MHz for this one.
    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.sysclk(48.mhz()).freeze();

    let delay = Delay::new(cp.SYST, clocks);

    // Set up I2C - SCL is PB8 and SDA is PB9; they are set to Alternate Function 4
    // as per the STM32F446xC/E datasheet page 60. Pin assignment as per the Nucleo-F446 board.
    let gpiob = dp.GPIOB.split();
    let scl = gpiob.pb8.into_alternate_af4().set_open_drain();
    let sda = gpiob.pb9.into_alternate_af4().set_open_drain();
    let i2c = I2c::i2c1(dp.I2C1, (scl, sda), 400.khz(), clocks);

    let mut grideye = GridEye::new(i2c, delay, Address::Standard);
    grideye.power(Power::Wakeup).unwrap();

    loop {
        for x in 0..8 {
            for y in 0..8 {
                let pixel = (x * 8) + y;
                let _temp = grideye.get_pixel_temperature_celsius(pixel).unwrap();
            }
        }
    }
}

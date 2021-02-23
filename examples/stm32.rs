// ------------------------------------------------------------------------------
// Copyright 2018 Uwe Arzt, mail@uwe-arzt.de
// SPDX-License-Identifier: Apache-2.0
// ------------------------------------------------------------------------------
// int,scl,sda
// pa15,pb8,pb9

// some interrupt tricks in here?
// https://github.com/jamesdanielv/thermalcam/blob/master/Adafruit_AMG88xx.cpp

// ・To have more than 4°C of temperature difference from background
// ・Detection object size:700×250mm(Assumable human body size)

#![no_main]
#![no_std]
#![feature(array_chunks)]

use grideye::{temperature_u12_to_f32_celsius, Address, GridEye, Power};
use rtt_target::{rprintln, rtt_init_print};
use stm32f4xx_hal::{delay::Delay, i2c::I2c, prelude::*, stm32 as hal};
type N = heapless::consts::U64;

#[cortex_m_rt::entry]
fn main() -> ! {
    rtt_init_print!(BlockIfFull, 128);

    let dp = hal::Peripherals::take().unwrap();
    let cp = cortex_m::peripheral::Peripherals::take().unwrap();

    // Set up the system clock. We want to run at 48MHz for this one.
    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.use_hse(25.mhz()).sysclk(100.mhz()).freeze();

    let mut delay = Delay::new(cp.SYST, clocks);

    let gpiob = dp.GPIOB.split();
    let scl = gpiob.pb8.into_alternate_af4().set_open_drain();
    let sda = gpiob.pb9.into_alternate_af4().set_open_drain();
    let i2c = I2c::i2c1(dp.I2C1, (scl, sda), 400.khz(), clocks);

    let mut grideye = GridEye::new(i2c, Address::Standard);
    // 50ms startup time
    delay.delay_ms(50_u16);

    grideye.power(Power::Wakeup).unwrap();
    grideye.reset().unwrap();
    grideye.set_framerate(grideye::Framerate::Fps1).unwrap();

    // wait for first reading, or skip this and be ok with all zeros
    // note readings arent actually stable for 15seconds after setup anyway...
    delay.delay_ms(1000_u16);

    // get the device temperature
    rprintln!(
        "device temperature: {}",
        grideye.get_device_temperature_celsius().unwrap()
    );

    let mut pixels = [0u8; 128];

    // process before sending to print
    loop {
        grideye.get_pixels_temperature_raw(&mut pixels).unwrap();

        //group by 2 u8s, turn into u16, then call temperature_u12_to_f32_celsius-> f32
        let out = pixels
            .array_chunks::<2>()
            .map(|chunk| u16::from_le_bytes(*chunk))
            .map(|raw| temperature_u12_to_f32_celsius(raw, 0.25))
            .collect::<heapless::Vec<f32, N>>();

        rprintln!("{:?}", &out);
        delay.delay_ms(1000_u16);
    }
}

// if an panic happens, print it out and signal probe-run to exit
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    rprintln!("{}", info);
    loop {
        cortex_m::asm::bkpt() // halt = exit probe-run
    }
}

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

// ssd1306 in spi mode (r3,r4 on back)
// https://github.com/jandelgado/arduino/wiki/SSD1306-based-OLED-connected-to-Arduino
// D0	SCL,CLK,SCK	Clock   b13 sck2
// D1	SDA,MOSI	Data    b15 mosi2
// N/A  MISO                b14 miso2
// RES	RST,RESET	Rest    a8
// DC	A0	Data/Command    a9
// CS	Chip Select         a10 // unused

#![no_main]
#![no_std]
#![feature(array_chunks)]

use hal::prelude::*;
use hal::stm32 as pac;
use rtt_target::{rprintln, rtt_init_print};
use stm32f4xx_hal as hal;

use grideye::{temperature_u12_to_f32_celsius, Address, GridEye, Power};
use hal::delay::Delay;
use hal::i2c::I2c;

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::style::PrimitiveStyle;
use hal::spi::{Mode, Phase, Polarity, Spi};
use smart_leds::hsv::{hsv2rgb, Hsv};
use ssd1331::{DisplayRotation, Ssd1331};

const SIZE: i32 = 8;
const GLOBAL_X_OFFSET: i32 = 30;
const GLOBAL_Y_OFFSET: i32 = 0;

#[cortex_m_rt::entry]
fn main() -> ! {
    rtt_init_print!(BlockIfFull, 128);

    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::peripheral::Peripherals::take().unwrap();

    // Set up the system clock. We want to run at 48MHz for this one.
    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.use_hse(25.mhz()).sysclk(100.mhz()).freeze();

    let mut delay = Delay::new(cp.SYST, clocks);

    let gpiob = dp.GPIOB.split();
    let scl = gpiob.pb8.into_alternate_af4().set_open_drain();
    let sda = gpiob.pb9.into_alternate_af4().set_open_drain();
    let i2c = I2c::i2c1(dp.I2C1, (scl, sda), 400.khz(), clocks);

    let gpioa = dp.GPIOA.split();

    let sck = gpiob.pb13.into_alternate_af5();
    let miso = gpiob.pb14.into_alternate_af5();
    let mosi = gpiob.pb15.into_alternate_af5();

    let mut rst = gpioa.pa8.into_push_pull_output();
    let dc = gpioa.pa9.into_push_pull_output();

    let spi = Spi::spi2(
        dp.SPI2,
        (sck, miso, mosi),
        Mode {
            polarity: Polarity::IdleLow,
            phase: Phase::CaptureOnFirstTransition,
        },
        stm32f4xx_hal::time::MegaHertz(8).into(),
        clocks,
    );

    // Set up the display
    let mut display = Ssd1331::new(spi, dc, DisplayRotation::Rotate0);
    display.reset(&mut rst, &mut delay).unwrap();
    display.init().unwrap();

    let mut grideye = GridEye::new(i2c, Address::Standard);
    // 50ms startup time
    delay.delay_ms(50_u16);

    grideye.power(Power::Wakeup).unwrap();
    grideye.reset().unwrap();
    grideye.set_framerate(grideye::Framerate::Fps10).unwrap();

    // wait for first reading, or skip this and be ok with all zeros
    // note readings arent actually stable for 15seconds after setup anyway...
    delay.delay_ms(1000_u16);

    // get the device temperature
    rprintln!(
        "device temperature: {}",
        grideye.get_device_temperature_celsius().unwrap()
    );

    let mut pixels = [0u8; 128];

    loop {
        // process before sending to print
        grideye.get_pixels_temperature_raw(&mut pixels).unwrap();

        //group by 2 u8s, turn into u16, then call temperature_u12_to_f32_celsius-> f32
        pixels
            .array_chunks::<2>()
            .map(|chunk| u16::from_le_bytes(*chunk))
            .map(|raw| temperature_u12_to_f32_celsius(raw, 0.25))
            .enumerate()
            .map(|(i, val)| {
                let i = i as i32;
                let xindex = i / 8;
                let yindex = i % 8;

                let xoffset = xindex * SIZE + GLOBAL_X_OFFSET;
                let yoffset = yindex * SIZE + GLOBAL_Y_OFFSET;

                (xoffset, yoffset, val)
            })
            .for_each(|(x, y, val)| {
                // between like .. 16 and 40s? generally? nightly saturating mul?
                let val = 255 - (val as u8 * 5);

                let blah = hsv2rgb(Hsv {
                    hue: 255,
                    sat: 128,
                    val,
                });

                Rectangle::new(Point::new(x, y), Point::new(x + SIZE, y + SIZE))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::new(
                        blah.r, blah.g, blah.b,
                    )))
                    .draw(&mut display)
                    .unwrap();
            });

        display.flush().unwrap();
        delay.delay_ms(100_u16);
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

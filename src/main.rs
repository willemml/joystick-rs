#![no_std]
#![no_main]

extern crate cortex_m;
extern crate panic_halt;
extern crate stm32f3xx_hal as hal;
extern crate usb_device;
extern crate usbd_hid;

use hal::hal::digital::v2::OutputPin;
use hal::pac::Peripherals;
use hal::prelude::*;
use hal::time::rate::Megahertz;

use hal::usb::{Peripheral, UsbBus};

use usb_device::prelude::*;

use usbd_hid::descriptor::generator_prelude::*;
use usbd_hid::descriptor::MouseReport;
use usbd_hid::hid_class::HIDClass;

use cortex_m::asm::delay;

use cortex_m_rt::entry;

#[entry]
fn main() -> ! {
    let dp = Peripherals::take().unwrap();

    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();

    let clocks = rcc
        .cfgr
        .use_hse(Megahertz(8))
        .sysclk(Megahertz(48))
        .pclk1(Megahertz(24))
        .pclk2(Megahertz(24))
        .freeze(&mut flash.acr);

    assert!(clocks.usbclk_valid());

    // Configure the on-board LED (LD10, south red)
    let mut gpioe = dp.GPIOE.split(&mut rcc.ahb);
    let mut led = gpioe
        .pe13
        .into_push_pull_output(&mut gpioe.moder, &mut gpioe.otyper);
    led.set_low().ok(); // Turn off

    let mut gpioa = dp.GPIOA.split(&mut rcc.ahb);

    // F3 Discovery board has a pull-up resistor on the D+ line.
    // Pull the D+ pin down to send a RESET condition to the USB bus.
    // This forced reset is needed only for development, without it host
    // will not reset your device when you upload new firmware.
    let mut usb_dp = gpioa
        .pa12
        .into_push_pull_output(&mut gpioa.moder, &mut gpioa.otyper);
    usb_dp.set_low().ok();
    delay(clocks.sysclk().0 / 100);

    let usb_dm =
        gpioa
            .pa11
            .into_af14_push_pull(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrh);
    let usb_dp = usb_dp.into_af14_push_pull(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrh);

    let usb = Peripheral {
        usb: dp.USB,
        pin_dm: usb_dm,
        pin_dp: usb_dp,
    };
    let usb_bus = UsbBus::new(usb);
    let mut usb_hid = HIDClass::new(&usb_bus, MouseReport::desc(), 60);
    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .manufacturer("Fake company")
        .product("Twitchy Mousey")
        .serial_number("TWTC")
        .device_class(0xEF)
        .build();

    loop {
        if !usb_dev.poll(&mut [&mut usb_hid]) {
            led.set_high().ok();
            continue;
        }

        usb_hid
            .push_input(&MouseReport {
                x: 0,
                y: 4,
                buttons: 0,
                wheel: 0,
            })
            .ok();

        led.set_low().ok();
    }
}

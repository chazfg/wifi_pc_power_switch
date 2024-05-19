#![no_std]
#![no_main]
pub mod request_handler;
pub mod write_resp_utils;
pub mod action_handler;

use request_handler::{ParsingResult, ParsingError, parse_request};
use write_resp_utils::{write_200, write_400};
use action_handler::{dispatch_action, handle_action};

use esp_hal::{
    clock::ClockControl,
    rng::Rng,
    prelude::*,
    // efuse::Efuse,
    delay::Delay,
    gpio::{IO, Output, GpioPin},
    timer::TimerGroup,
    peripherals::Peripherals,
};

use esp_println::println;
use embedded_io::*;
use esp_wifi::wifi::{ClientConfiguration, Configuration};
use serde::Deserialize;
use alloc::string::String;
use alloc::format;
use esp_backtrace as _;

use esp_wifi::{
    wifi::utils::create_network_interface,
    wifi::{WifiStaDevice, AccessPointInfo, WifiError, AuthMethod},
    wifi_interface::WifiStack,
    current_millis,
};
use core::str::FromStr;

use smoltcp::iface::SocketStorage;
// use httparse;


extern crate alloc;
use core::mem::MaybeUninit;

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

fn init_heap() {
    const HEAP_SIZE: usize = 32 * 1024;
    static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();

    unsafe {
        ALLOCATOR.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE);
    }
}

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");
const RX_BUFFER_SIZE: usize = 16384;
const TX_BUFFER_SIZE: usize = 16384;

#[entry]
fn main() -> ! {


    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();

    init_heap();

    let delay = Delay::new(&clocks);

    // #[cfg(target_arch = "xtensa")]
    let timer = TimerGroup::new(peripherals.TIMG1, &clocks, None).timer0;
    // #[cfg(target_arch = "riscv32")]
    // let timer = SystemTimer::new(peripherals.SYSTIMER).alarm0;
    let init = esp_wifi::initialize(
        esp_wifi::EspWifiInitFor::Wifi,
        timer,
        Rng::new(peripherals.RNG),
        system.radio_clock_control,
        &clocks,
    )
    .unwrap();
    let wifi = peripherals.WIFI;

    let mut socket_set_entries: [SocketStorage; 3] = Default::default();
    let (iface, device, mut controller, sockets) =
        create_network_interface(&init, wifi, WifiStaDevice, &mut socket_set_entries).unwrap();
    let wifi_stack = WifiStack::new(iface, device, sockets, current_millis);

    let client_config = Configuration::Client(ClientConfiguration {
        ssid: SSID.try_into().unwrap(),
        password: PASSWORD.try_into().unwrap(),
        // auth_method: AuthMethod::WPA2Personal,
        ..Default::default()
    });

    let res = controller.set_configuration(&client_config);
    println!("wifi_set_configuration returned {:?}", res);

    delay.delay(500.millis());
    controller.start().unwrap();
    println!("is wifi started: {:?}", controller.is_started());


    println!("Start Wifi Scan");
    let res: Result<(heapless::Vec<AccessPointInfo, 10>, usize), WifiError> = controller.scan_n();

    println!("{:?}", controller.get_capabilities());
    println!("wifi_connect {:?}", controller.connect());
    println!("Wait to get connected");

    loop {
        let res = controller.is_connected();
        match res {
            Ok(connected) => {
                if connected {
                    break;
                }
            }
            Err(err) => {
                println!("{:?}", err);
                loop {}
            }
        }
    }

    println!("{:?}", controller.is_connected());    // wait for getting an ip address
    
    println!("Wait to get an ip address");

    println!("iface config {:?}", wifi_stack.get_iface_configuration());

    loop {
        wifi_stack.work();

        if wifi_stack.is_iface_up() {
            println!("got ip {:?}", wifi_stack.get_ip_info());
            break;
        }
    }
    println!("creating buffer");

    let mut rx_buffer = [0u8; RX_BUFFER_SIZE];
    let mut tx_buffer = [0u8; TX_BUFFER_SIZE];
    let mut socket = wifi_stack.get_socket(&mut rx_buffer, &mut tx_buffer);
    println!("got socket");
    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let mut pwr_switch = io.pins.gpio21.into_push_pull_output();
    pwr_switch.set_high();

    println!("start work socket");
    socket.work();
    println!("socket worked");

    let mut buffer = [0u8; 2048];

    let mut app_io = ApplicationIo {
        power_switch: pwr_switch,
        delay: delay
    };

    loop {
    println!("listening");

        socket.listen(8080).unwrap();
        app_io.delay.delay(500.millis());
        socket.work();
        let wait_end = current_millis() + 20 * 1000;
        
        // TODO make this result 
        let w: Result<ParsingResult, ParsingError> = parse_request(
            // ideally we'd read right off the instream to handle but this works for now
            match socket.read(&mut buffer) {
                Ok(len) => &buffer[..len],
                Err(_) => b"yeah it's fuck"
            }
        );

        let response_to_send = match w {

            Ok(ParsingResult {response: res, action: Some(action)}) => handle_action(action, &mut app_io),
            Ok(resulting_action) => write_200(resulting_action.response),
            Err(error_parsed) => write_400(error_parsed)

        };

        // println!("{:?}", w);

        socket
            .write(response_to_send.as_bytes())
            .unwrap();

        socket.flush().unwrap();

        if current_millis() > wait_end {
            println!("Timeout");
        }

        socket.close(); socket.disconnect(); socket.work();

        buffer = [0u8; 2048];
    }

        
}


pub struct ApplicationIo {

    pub power_switch: GpioPin<Output<esp_hal::gpio::PushPull>, 21>,
    pub delay: Delay

}

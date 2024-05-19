#![no_std]
#![no_main]
pub mod request_handler;
mod action_handler;

use request_handler::{ParsingResult, ParsingError, parse_request};
use action_handler::dispatch_action;

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

// use alloc;

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
// use esp_hal::ledc::{
//     LEDC,
//     channel,
//     timer::HSClockSource,
//     timer,
//     HighSpeed,
//     channel::config::PinConfig,
// };

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
    // if let Ok((res, _count)) = res {
    //     for ap in res {
    //         println!("{:?}", ap);
    //     }
    // }

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

            Ok(ParsingResult {response: res, action: Some(action)}) => {
                dispatch_action(action, &mut app_io);
                write_200(String::from("some(action)"))
            },
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


// TODO make this take in a reference to a socket and perform the writing in the function
fn write_200(response: String) -> String {
    
    format!(
        "HTTP/1.0 200 OK\r\n\
        Content-Type: text/html\r\n\
        Content-Length: {}\r\n\
        \r\n\
        {}\r\n\
        ", response.len(), response)


}

fn write_400(error_parsed: ParsingError) -> String {

    // TODO: specify code/message based on error parsed
    String::from(
        "HTTP/1.0 400 Bad Request\r\n\
        Content-Type: application/problem+json\r\n\
        \r\n\
        {\r\n\
            \"title\": \"Error while handling request\",\r\n\
            \"status\": 400,\r\n\
        }\r\n\
        ")

}

pub struct ApplicationIo {

    pub power_switch: GpioPin<Output<esp_hal::gpio::PushPull>, 21>,
    pub delay: Delay

}

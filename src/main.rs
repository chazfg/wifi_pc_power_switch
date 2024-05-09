#![no_std]
#![no_main]

use embedded_io::*;
use esp_wifi::wifi::{ClientConfiguration, Configuration};
use serde::Deserialize;

// extern crate alloc;

// use core::mem::MaybeUninit;
use alloc::{
    vec::Vec,
    format};
// use alloc::vec::Vec;
use alloc::string::String;
use esp_backtrace as _;
use esp_println::println;
use esp_wifi::{
    initialize,
    wifi::utils::create_network_interface,
    wifi::{WifiStaDevice, AccessPointInfo, WifiError, AuthMethod},
    wifi_interface::WifiStack,
    current_millis,
    EspWifiInitFor,
};
use esp_hal::ledc::{
    LEDC,
    channel,
    timer::HSClockSource,
    timer,
    HighSpeed,
    channel::config::PinConfig,
};
// use crate::alloc::string::ToString;
use embedded_hal::pwm;
use esp_hal::{
    clock::ClockControl,
    rng::Rng,
    efuse::Efuse,
    delay::Delay,
    gpio::IO,
    // cpu_control::{CpuControl, Stack},
    prelude::*,
    timer::TimerGroup,
    peripherals::Peripherals,
};
use core::str::FromStr;

// use smoltcp::wire::IpAddress;
use smoltcp::iface::SocketStorage;
use httparse;

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");
// const HOST_IP: &str = "192.168.1.1";

// const TEST_DURATION: usize = 15;
const RX_BUFFER_SIZE: usize = 16384;
const TX_BUFFER_SIZE: usize = 16384;
// const IO_BUFFER_SIZE: usize = 1024;
// const DOWNLOAD_PORT: u16 = 4321;
// const UPLOAD_PORT: u16 = 4322;
// const UPLOAD_DOWNLOAD_PORT: u16 = 4323;

// start       end
// 0×3F80_0000 0×3FBF_FFFF

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

#[entry]
fn main() -> ! {
    #[cfg(feature = "log")]
    esp_println::logger::init_logger(log::LevelFilter::Info);


    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();


        let ledc = LEDC::new(peripherals.LEDC, &clocks);

    let mut hstimer0 = ledc.get_timer::<HighSpeed>(timer::Number::Timer0);
    hstimer0
      .configure(timer::config::Config {
          duty: timer::config::Duty::Duty5Bit,
          clock_source: HSClockSource::APBClk,
          frequency: 24.kHz(),
      })
      .unwrap();



    let delay = Delay::new(&clocks);
    init_heap();

    #[cfg(target_arch = "xtensa")]
    let timer = TimerGroup::new(peripherals.TIMG1, &clocks, None).timer0;
    #[cfg(target_arch = "riscv32")]
    let timer = SystemTimer::new(peripherals.SYSTIMER).alarm0;
    let init = initialize(
        EspWifiInitFor::Wifi,
        timer,
        Rng::new(peripherals.RNG),
        system.radio_clock_control,
        &clocks,
    )
    .unwrap();

    let wifi = peripherals.WIFI;

    println!("{:?}", Efuse::get_mac_address());
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
    if let Ok((res, _count)) = res {
        for ap in res {
            println!("{:?}", ap);
        }
    }

    println!("{:?}", controller.get_capabilities());
    println!("wifi_connect {:?}", controller.connect());

    // wait to get connected
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
    println!("{:?}", controller.is_connected());

    // wait for getting an ip address
    println!("Wait to get an ip address");
    // let mut counter = 0u16;

    println!("{:?}", wifi_stack.get_iface_configuration());
    loop {
        wifi_stack.work();

        if wifi_stack.is_iface_up() {
            println!("got ip {:?}", wifi_stack.get_ip_info());
            break;
        }
        // counter += 1;

    }

    let mut rx_buffer = [0u8; RX_BUFFER_SIZE];
    let mut tx_buffer = [0u8; TX_BUFFER_SIZE];
    let mut socket = wifi_stack.get_socket(&mut rx_buffer, &mut tx_buffer);
    println!("got socket");
   let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let mut pwr_switch = io.pins.gpio21.into_push_pull_output();
    let mut led = io.pins.gpio4.into_push_pull_output();


      let mut channel0 = ledc.get_channel(channel::Number::Channel0, led);
  channel0
      .configure(channel::config::Config {
          timer: &hstimer0,
          duty_pct: 10,
          pin_config: PinConfig::PushPull,

      })
      .unwrap();

    pwr_switch.set_high();

    println!("start work socket");
    socket.work();
    println!("socket worked");

    let mut buffer = [0u8; 2048];
    loop {
    println!("listening");

        socket.listen(8080).unwrap();
        delay.delay(500.millis());
        socket.work();
        let wait_end = current_millis() + 20 * 1000;
        
        let w = parse_request(
            // ideally we'd read right off the instream to handle but this works for now
            match socket.read(&mut buffer) {
                Ok(len) => &buffer[..len],
                Err(_) => b"yeah it's fuck"
            }
        );
        pwr_switch.toggle();
        delay.delay(300.millis());
        pwr_switch.toggle();
        socket
            .write(w.as_bytes())
            .unwrap();

        socket.flush().unwrap();

        if current_millis() > wait_end {
            println!("Timeout");
        }

        socket.close(); socket.disconnect(); socket.work();

        buffer = [0u8; 2048];
    }

        
}

// struct Request {
    // method: 
    // endpoint:
    // protocol:
    // headers:
    // content:
// }

fn parse_request(req: &[u8]) -> String {

    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut req_parse = httparse::Request::new(&mut headers);
    match req_parse.parse(req) {
        Ok(httparse::Status::Complete(offset)) => println!("{}", match_request_method(req_parse, offset, req)),
        Ok(httparse::Status::Partial) => println!("got partial"),
        Err(_) => println!("ouch")
    };


    match req {
        _ => println!("unhandled\n{:?}", req)
    };    


    let write_out = "<html>\
        <body>\
        <h1>Hello Rust! Hello esp-wifi!</h1>\
        </body>\
        </html>";

    format!(
        "HTTP/1.0 200 OK\r\n\
        Content-Type: text/html\r\n\
        Content-Length: {}\r\n\
        \r\n\
        {}\r\n\
        ", write_out.len(), write_out)

}


fn match_request_method(parsed_request: httparse::Request, offset: usize, req: &[u8]) -> String {
    match parsed_request.method {
        Some("GET") => handle_get_request(parsed_request).response,
        Some("POST") => handle_post_request(parsed_request, offset, req),
        _ => String::from("dinnae get that")
    }
}

fn handle_get_request(get_request: httparse::Request) -> ParsingResult {
    let r = match get_request.path {
        Some("/kep") => String::from("is kep"),
        _ => String::from("is noo kep")
    };

    ParsingResult::new(r, None)
}


fn handle_post_request(post_request: httparse::Request, offset: usize, req: &[u8]) -> String {
    println!("{:?}", post_request);
    // println!("{:?}", core::str::from_utf8(&req[offset..]));

    match post_request.path {
        Some("/ring_light") => handle_ring_light(&req[offset..]),
        _ => String::from("is noo kep")
    }
}

struct ParsingResult {
    response: String,
    action: Option<Action>
}

#[derive(Deserialize, Debug)]
#[serde(tag = "action")]
enum Action {
    #[serde(rename="set_duty_cycle")]
    SetDutyCycle {value: i32}
}

impl ParsingResult {
    fn new(response: String, action: Option<Action>) -> Self {
        Self { response, action }
    }
}



fn handle_ring_light(request_content: &[u8]) -> String {
    let found_action: Action = serde_json::from_slice(request_content).expect("should deserial");
    
    println!("{:?}", found_action);


    String::from("lkajdsf")
}





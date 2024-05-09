#![no_std]
#![no_main]

use embedded_io::*;
use esp_wifi::wifi::{ClientConfiguration, Configuration};


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

// use smoltcp::wire::IpAddress;
use smoltcp::iface::SocketStorage;

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
    // led2.set_high();
    // led2.set_duty_cycle_percent(80u8);
    // delay.delay(500.millis());
    // led.set_high();
    // delay.delay(500.millis());
    // led.set_low();
    // delay.delay(500.millis());
    // led.set_high();
    // delay.delay(500.millis());
    // led.set_low();


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
        
        let w = handle_request(
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

// struct Request

fn handle_request(req: &[u8]) -> String {

    let mut buff_str: String = String::from("");
    core::str::from_utf8(req)
        .unwrap()
        .chars()
        .for_each(|c| {
            match c {
                '\r' | ' ' => {println!("{:?}", buff_str); buff_str.clear();},
                '\n' => (),
                _ => buff_str.push(c)
            }
        });
    
    // match  header {
    //     ["GET", ..] => println!("gets {:?}", header),
    //     _ => println!("not gets");
    // }


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


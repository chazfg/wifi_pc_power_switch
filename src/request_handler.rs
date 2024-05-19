use serde::Deserialize;
// #![no_std]
// use core::{
    // writeln,
    // convert::From,
    // option::Option,
    // option::Option::{Some, None},
    // result::Result::{Ok, Err}
// };
use esp_println::println;
use httparse;
use alloc::{
    string::String,
    vec::Vec,
    format};


pub enum ParsingError {
    Unspecified,
    PartialInputReceived,
    PathNotFound
}

pub fn parse_request(req: &[u8]) -> Result<ParsingResult, ParsingError> {

    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut req_parse = httparse::Request::new(&mut headers);

    match req_parse.parse(req) {
        Ok(httparse::Status::Complete(offset)) => Ok(match_request_method(req_parse, offset, req)?),
        Ok(httparse::Status::Partial) => Err(ParsingError::PartialInputReceived),
        Err(_) => Err(ParsingError::Unspecified)
    }


    // match req {
    //     _ => println!("unhandled\n")
    // };    


    // let write_out = "<html>\
    //     <body>\
    //     <h1>Hello Rust! Hello esp-wifi!</h1>\
    //     </body>\
    //     </html>";

    // format!(
    //     "HTTP/1.0 200 OK\r\n\
    //     Content-Type: text/html\r\n\
    //     Content-Length: {}\r\n\
    //     \r\n\
    //     {}\r\n\
    //     ", write_out.len(), write_out)

}


fn match_request_method(parsed_request: httparse::Request, offset: usize, req: &[u8]) -> Result<ParsingResult, ParsingError> {
    match parsed_request.method {
        Some("GET") => Ok(handle_get_request(parsed_request)?),
        Some("POST") => Ok(handle_post_request(parsed_request, offset, req)?),
        _ => Err(ParsingError::Unspecified)
    }
}

fn handle_get_request(get_request: httparse::Request) -> Result<ParsingResult, ParsingError> {
    match get_request.path {
        Some("/kep") => Ok(ParsingResult::new(String::from("is kep"), None)),
        _ => Err(ParsingError::PathNotFound)
    }

    // ParsingResult::new(r, None)
}


fn handle_post_request(post_request: httparse::Request, offset: usize, req: &[u8]) -> Result<ParsingResult, ParsingError> {
    println!("{:?}", post_request);
    // println!("{:?}", core::str::from_utf8(&req[offset..]));

    // middleware to handle general auth here

    match post_request.path {
        Some("/pc_switch") => Ok(ParsingResult::new(String::from("Toggle switch action received"), Some(Action::TogglePcPowerSwitch))),     // each handler should handle it's own auth?
        Some("/ring_light") => Ok(handle_ring_light(&req[offset..])),
        _ => Err(ParsingError::PathNotFound)
    }
}



pub struct ParsingResult {
    pub response: String,
    pub action: Option<Action>
}

#[derive(Deserialize, Debug)]
#[serde(tag = "action")]
pub enum Action {
    #[serde(rename="set_duty_cycle")]
    SetDutyCycle {value: i32},
    #[serde(rename="toggle_pc_power_switch")]
    TogglePcPowerSwitch
}

impl ParsingResult {
    fn new(response: String, action: Option<Action>) -> Self {
        Self { response, action }
    }
}



fn handle_ring_light(request_content: &[u8]) -> ParsingResult {
    ParsingResult::new(
        String::from(""),
        Some(serde_json::from_slice(request_content).expect("should deserial"))
    )
}
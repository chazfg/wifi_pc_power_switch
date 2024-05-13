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

pub fn parse_request(req: &[u8]) -> String {

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
        Some("POST") => handle_post_request(parsed_request, offset, req).response,
        _ => String::from("dinnae get that")
    }
}

fn handle_get_request(get_request: httparse::Request) -> ParsingResult {
    match get_request.path {
        Some("/kep") => ParsingResult::new(String::from("is kep"), None),
        _ => ParsingResult::new(String::from("is noo kep"), None)
    }

    // ParsingResult::new(r, None)
}


fn handle_post_request(post_request: httparse::Request, offset: usize, req: &[u8]) -> ParsingResult {
    println!("{:?}", post_request);
    // println!("{:?}", core::str::from_utf8(&req[offset..]));

    match post_request.path {
        Some("/ring_light") => handle_ring_light(&req[offset..]),
        _ => ParsingResult::new(String::from("is noo kep"), None)
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



fn handle_ring_light(request_content: &[u8]) -> ParsingResult {
    ParsingResult::new(
        String::from(""),
        Some(serde_json::from_slice(request_content).expect("should deserial"))
    )
}
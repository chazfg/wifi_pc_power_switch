use esp_println::println;
use crate::ParsingError;
use crate::action_handler::ActionError;
use alloc::{
    string::String,
    format
};
// TODO make this take in a reference to a socket and perform the writing in the function
pub fn write_200(response: String) -> String {
    
    format!(
        "HTTP/1.0 200 OK\r\n\
        Content-Type: text/html\r\n\
        Content-Length: {}\r\n\
        \r\n\
        {}\r\n\
        ", response.len(), response)


}

// have to make this more general 
pub fn write_400(error_parsed: ParsingError) -> String {

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

pub fn write_400_from_string(error: String) -> String {

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

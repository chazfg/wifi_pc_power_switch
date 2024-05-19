use esp_println::println;
use crate::request_handler::Action;
use crate::ApplicationIo;
use esp_hal::gpio::{GpioPin, Output};
use esp_hal::delay::Delay;
use crate::write_resp_utils::{write_200, write_400_from_string};
use alloc::{
	string::String,
	format
};

pub enum ActionError {
	Unspecified,
	PowerSwitchToggleError
}


pub fn handle_action(requested_action: Action, app_io: &mut ApplicationIo) -> String {

    match dispatch_action(requested_action, app_io) {
    	Ok(_) => write_200(String::from("Action Completed")),
    	Err(_) => write_400_from_string(String::from("Error completing action"))
    }

}


// todo: make the pwr_switch_pin take more general argument
// this should be able to dispatch *any* action that needs to be dispatched
pub fn dispatch_action(requested_action: Action, app_io: &mut ApplicationIo) -> Result<(), ActionError> {

	match requested_action {
		Action::TogglePcPowerSwitch => toggle_pin_power(app_io)?,
		_ => println!("doing nothing")
	};

	Ok(())


}


fn toggle_pin_power(app_io: &mut ApplicationIo) -> Result<(), ActionError> {

	app_io.power_switch.toggle();
	app_io.delay.delay_millis(300);	
	app_io.power_switch.toggle();

	Ok(())

}
// figure out how to just set the pin from a raw pointer

// move to 'action' function

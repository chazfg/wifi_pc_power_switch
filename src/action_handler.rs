use esp_println::println;
use crate::request_handler::Action;
use crate::ApplicationIo;
use esp_hal::gpio::{GpioPin, Output};
use esp_hal::delay::Delay;

pub enum ActionError {
	Unspecified,
	PowerSwitchToggleError
}

// todo: make the pwr_switch_pin take more general argument
// todo: make state struct? Something that generalizes the input into the action
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

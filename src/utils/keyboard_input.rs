use opencv::highgui;

use crate::config::Config;

pub enum KeyboardInput {
    NextFrame,
    PreviousFrame,
    NextVideo,
    Quit,
}

/// Wait for user input to continue or quit. Returns false if user inputs "q".
/// If `autoplay` is enabled in the config, it will return true immediately.
pub fn wait_for_keyboard_input(config: &Config) -> Result<KeyboardInput, opencv::Error> {
    if config.visualization.autoplay {
        if highgui::wait_key(1)? == 113 {
            return Ok(KeyboardInput::Quit);
        }

        return Ok(KeyboardInput::NextFrame);
    }

    match highgui::wait_key(0)? {
        113 => Ok(KeyboardInput::Quit),         // q
        39 => Ok(KeyboardInput::NextFrame),     // right arrow
        37 => Ok(KeyboardInput::PreviousFrame), // left arrow
        40 => Ok(KeyboardInput::NextVideo),     // down arrow
        32 => Ok(KeyboardInput::NextVideo),     // space
        _ => Ok(KeyboardInput::NextFrame),
    }
}

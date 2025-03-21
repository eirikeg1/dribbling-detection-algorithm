use opencv::highgui;

use crate::config::Config;

#[derive(Debug)]
pub enum KeyboardInput {
    NextFrame,
    PreviousFrame,
    NextClip,
    Quit,
    Dribble,
    Tackle,
    None,
}

/// Parse OpenCV key code to KeyboardInput enum.
/// If `filtered_keys` is set, only the provided key codes are processed.
/// Otherwise, all key codes are processed.
///
/// Key codes are mapped as follows:
///  - q: Quit
///  - right arrow: NextFrame
///  - left arrow: PreviousFrame
///  - down arrow: NextClip
///  - space: NextClip
///  - d: Dribble
///  - t: Tackle
///  - n: None
/// Any other key is ignored and returns NextFrame.
fn parse_input_code(
    code: opencv::Result<i32>,
    filtered_keys: Option<&[i32]>,
) -> Result<KeyboardInput, opencv::Error> {
    let key_code = code?;

    // If filtered_keys is provided, only process key if it's in the allowed list
    if let Some(allowed_keys) = filtered_keys {
        if !allowed_keys.contains(&key_code) {
            return Ok(KeyboardInput::NextFrame);
        }
    }

    match key_code {
        113 => Ok(KeyboardInput::Quit),         // q
        39 => Ok(KeyboardInput::NextFrame),     // right arrow
        37 => Ok(KeyboardInput::PreviousFrame), // left arrow
        40 => Ok(KeyboardInput::NextClip),      // down arrow
        32 => Ok(KeyboardInput::NextClip),      // space
        100 => Ok(KeyboardInput::Dribble),      // d
        116 => Ok(KeyboardInput::Tackle),       // t
        110 => Ok(KeyboardInput::None),         // n
        _ => Ok(KeyboardInput::NextFrame),
    }
}

/// Wait for user input. If autoplay is on, it quickly returns NextFrame; otherwise it blocks.
/// Press:
///   - 'q' to quit,
///   - right/left arrow for next/prev,
///   - down arrow for next clip,
///   - d/t/n to label the clip.
pub fn wait_for_keyboard_input(config: &Config) -> opencv::Result<KeyboardInput> {
    if config.visualization.autoplay {
        // Autoplay => proceed automatically

        let wait_time = if config.general.video_mode == "display" {
            20
        } else {
            1
        };

        return parse_input_code(highgui::wait_key(wait_time), Some(&[32, 40, 113]));
    }

    parse_input_code(highgui::wait_key(0), None)
}

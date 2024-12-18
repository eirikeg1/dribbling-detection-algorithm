use opencv::core::{Mat, Size, StsError};
use opencv::imgproc;
use opencv::prelude::MatTraitConst;
use crate::config::Config;

pub fn scale_frame(frame: &mut Mat, config: &Config) -> opencv::Result<()> {
    let scale = config.visualization.scale_factor;
    let new_size = Size {
        width: (frame.cols() as f64 * scale) as i32,
        height: (frame.rows() as f64 * scale) as i32,
    };

    if new_size.width <= 0 || new_size.height <= 0 {
        return Err(opencv::Error::new(
            StsError,
            "Scaled frame size is invalid (non-positive dimensions).",
        ));
    }

    let mut resized_frame = Mat::default();
    imgproc::resize(
        frame,
        &mut resized_frame,
        new_size,
        0.0,
        0.0,
        imgproc::INTER_LINEAR,
    )?;

    *frame = resized_frame; // Update the original frame
    Ok(())
}
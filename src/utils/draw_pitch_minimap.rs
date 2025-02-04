use opencv::prelude::MatTraitConst;
use opencv::{
    core::{Mat, Point, Rect, Scalar},
    imgproc,
};

use crate::config::Config;

// Draw the pitch markings on the minimap. Includes center line, goal lines, and penalty areas
// Are based on standard pitch dimensions
pub fn draw_pitch_markings_on_minimap(minimap: &mut Mat, config: &Config) -> opencv::Result<()> {
    let minimap_width = minimap.cols();
    let minimap_height = minimap.rows();
    // Get pitch bounds from the config (assumed to be f64)
    let x_min = config.visualization.x_min;
    let x_max = config.visualization.x_max;
    let y_min = config.visualization.y_min;
    let y_max = config.visualization.y_max;
    let mid_x = (x_min + x_max) / 2.0;
    let mid_y = (y_min + y_max) / 2.0;

    if minimap_width >= minimap_height {
        // Draw the center line (vertical)
        let center_line_x = (((mid_x - x_min) / (x_max - x_min)) * minimap_width as f64) as i32;
        let pt1 = Point::new(center_line_x, 0);
        let pt2 = Point::new(center_line_x, minimap_height);
        imgproc::line(
            minimap,
            pt1,
            pt2,
            Scalar::new(255.0, 255.0, 255.0, 255.0), // white center line
            2,
            imgproc::LINE_8,
            0,
        )?;

        // Draw penalty boxes on the left and right
        // Use fractions similar to standard pitch dimensions:
        // For a 105m x 68m pitch the 16 meter mark is about 15.7% and the box height about 59.3%.
        // The penaly box is about 10% height
        let penalty_depth = (x_max - x_min) * 0.157;
        let penalty_box_height = (y_max - y_min) * 0.593;
        let box_top = mid_y + penalty_box_height / 2.0;
        let box_bottom = mid_y - penalty_box_height / 2.0;

        // Left penalty box: from x_min to x_min + penalty_depth
        let left_box_right = ((penalty_depth / (x_max - x_min)) * minimap_width as f64) as i32;
        let left_box_y1 = (((box_bottom - y_min) / (y_max - y_min)) * minimap_height as f64) as i32;
        let left_box_y2 = (((box_top - y_min) / (y_max - y_min)) * minimap_height as f64) as i32;
        let left_box = Rect::new(0, left_box_y1, left_box_right, left_box_y2 - left_box_y1);
        imgproc::rectangle(
            minimap,
            left_box,
            Scalar::new(255.0, 255.0, 255.0, 255.0), // white outline
            2,
            imgproc::LINE_8,
            0,
        )?;

        // Right penalty box: from x_max - penalty_depth to x_max
        let right_box_left =
            (((x_max - penalty_depth - x_min) / (x_max - x_min)) * minimap_width as f64) as i32;
        let right_box = Rect::new(
            right_box_left,
            left_box_y1,
            minimap_width - right_box_left,
            left_box_y2 - left_box_y1,
        );
        imgproc::rectangle(
            minimap,
            right_box,
            Scalar::new(255.0, 255.0, 255.0, 255.0),
            2,
            imgproc::LINE_8,
            0,
        )?;
    } else {
        // (If needed, add a branch here for vertical orientation.)
    }

    // Draw a thin black border around the pitch
    imgproc::rectangle(
        minimap,
        Rect::new(0, 0, minimap_width, minimap_height),
        Scalar::new(255.0, 255.0, 255.0, 255.0), // black
        1,
        imgproc::LINE_8,
        0,
    )?;

    Ok(())
}

use crate::config::Config;
use crate::domain::data::models::{Annotation, BboxImage};
use crate::domain::events::drible_models::DribbleEvent;
use opencv::core::{self, Mat, Rect, Scalar};
use opencv::imgproc;
use opencv::prelude::*;
use std::collections::HashMap;
use super::annotation_calculations::get_annotation_color;

pub fn draw_annotations(
    frame: &mut Mat,
    annotations: &[Annotation],
    categories: &HashMap<String, u32>,
    dribble_event: Option<DribbleEvent>,
    image_id: &str,
    config: &Config,
) -> opencv::Result<()> {
    let annotations: Vec<Annotation> = annotations
        .iter()
        .filter(|ann| ann.image_id == *image_id)
        .cloned()
        .collect();

    let scale_factor = config.visualization.scale_factor;

    // Draw bounding boxes in the main frame
    for annotation in &annotations {
        if let Some(bbox_image) = &annotation.bbox_image {
            draw_bbox_image(frame, bbox_image, scale_factor, get_annotation_color(annotation, categories))?;
        }
    }

    let minimap_height = config.visualization.minimap_height;
    let minimap_width = config.visualization.minimap_width;

    // Extend the original frame to fit minimap
    let extended_height = frame.rows() + minimap_height;
    let extended_width = frame.cols().max(minimap_width);
    let mut extended_frame = Mat::zeros(extended_height, extended_width, frame.typ())?.to_mat()?;

    // Copy main frame into the extended frame
    let roi_main = Rect::new(0, 0, frame.cols(), frame.rows());
    let mut extended_roi_main = Mat::roi_mut(&mut extended_frame, roi_main)?;
    frame.copy_to(&mut extended_roi_main)?;

    // Create a green minimap
    let mut minimap = Mat::zeros(minimap_height, minimap_width, frame.typ())?.to_mat()?;
    imgproc::rectangle(
        &mut minimap,
        Rect::new(0, 0, minimap_width, minimap_height),
        Scalar::new(69.0, 160.0, 40.0, 255.0), // Green
        -1,
        imgproc::LINE_8,
        0,
    )?;

    // If you have the outer radius in your config, retrieve it here
    let outer_rad = config.dribbling_detection.outer_radius;
    let inner_rad = config.dribbling_detection.inner_radius;
    let y_min = config.visualization.y_min;
    let y_max = config.visualization.y_max;
    let x_min = config.visualization.x_min;
    let x_max = config.visualization.x_max;

    // println!(" * Dribble event: {:?}", dribble_event);

    // Draw pitch points (and possibly the circle) onto the minimap
    for annotation in &annotations {
        if let Some(bbox_pitch) = &annotation.bbox_pitch {
            let is_possession_holder = match &dribble_event {
                Some(de) => de.possession_holder == annotation.track_id.unwrap(),
                None => false,
            };

            // Determine the color of the dot. If the annotation is the possession holder,
            // use yellow for the player marker
            let color = if is_possession_holder {
                Scalar::new(255.0, 255.0, 40.0, 255.0) // Bright yellow
            } else {
                get_annotation_color(annotation, categories)
            };

            // 1) Draw the dot for the player
            draw_pitch_point_on_minimap(
                &mut minimap,
                bbox_pitch.x_bottom_middle,
                bbox_pitch.y_bottom_middle,
                config,
                color,
            )?;

            // 2) If the annotation is the possession holder, draw the outer range circle
            if is_possession_holder {
                // Convert the player's pitch coords to minimap coords
                let mx = ((bbox_pitch.x_bottom_middle - x_min) / (x_max - x_min)
                    * minimap_width as f64) as i32;
                let my = ((bbox_pitch.y_bottom_middle - y_min) / (y_max - y_min)
                    * minimap_height as f64) as i32;

                // Convert the `outer_rad` (pitch space) into minimap space
                let inner_rad_x = (inner_rad / (x_max - x_min)) * minimap_width as f64;
                let inner_rad_y = (inner_rad / (y_max - y_min)) * minimap_height as f64;
                let outer_rad_x = (outer_rad / (x_max - x_min)) * minimap_width as f64;
                let outer_rad_y = (outer_rad / (y_max - y_min)) * minimap_height as f64;
                let inner_circle_radius = inner_rad_x.min(inner_rad_y) as i32;
                let outer_circle_radius = outer_rad_x.min(outer_rad_y) as i32;

                // Draw an outline circle (thickness=2) around the possession holder
                // to show the "outer range"
                imgproc::circle(
                    &mut minimap,
                    core::Point::new(mx, my),
                    outer_circle_radius,
                    Scalar::new(0.0, 255.0, 255.0, 255.0), // bright yellow outline
                    2,                                     // thickness
                    imgproc::LINE_8,
                    0,
                )?;
                // Draw the inner circle (e.g., orange)
                imgproc::circle(
                    &mut minimap,
                    core::Point::new(mx, my),
                    inner_circle_radius,
                    Scalar::new(0.0, 140.0, 255.0, 255.0), // orange
                    1,
                    imgproc::LINE_8,
                    0,
                )?;
            }
        }
    }

    // Place the minimap below the main frame
    let minimap_x_offset = (frame.cols() - minimap_width) / 2; // center horizontally
    let roi_minimap = Rect::new(
        minimap_x_offset,
        frame.rows(),
        minimap_width,
        minimap_height,
    );
    let mut extended_roi_minimap = Mat::roi_mut(&mut extended_frame, roi_minimap)?;
    minimap.copy_to(&mut extended_roi_minimap)?;

    // Overwrite original frame with extended frame
    *frame = extended_frame;

    Ok(())
}

fn draw_bbox_image(
    frame: &mut Mat,
    bbox_image: &BboxImage,
    scale: f64,
    color: Scalar,
) -> opencv::Result<()> {
    let x = (bbox_image.x as f64 * scale) as i32;
    let y = (bbox_image.y as f64 * scale) as i32;
    let w = (bbox_image.w as f64 * scale) as i32;
    let h = (bbox_image.h as f64 * scale) as i32;

    let rect = Rect::new(x, y, w, h);
    imgproc::rectangle(frame, rect, color, 1, imgproc::LINE_8, 0)?;
    Ok(())
}

fn draw_pitch_point_on_minimap(
    minimap: &mut Mat,
    pitch_x: f64,
    pitch_y: f64,
    config: &Config,
    color: Scalar,
) -> opencv::Result<()> {
    let minimap_height = config.visualization.minimap_height;
    let minimap_width = config.visualization.minimap_width;
    let y_min = config.visualization.y_min;
    let y_max = config.visualization.y_max;
    let x_min = config.visualization.x_min;
    let x_max = config.visualization.x_max;

    // Convert pitch coordinates to minimap coordinates
    let mx = ((pitch_x - x_min) / (x_max - x_min) * minimap_width as f64) as i32;
    let my = ((pitch_y - y_min) / (y_max - y_min) * minimap_height as f64) as i32;

    // Draw an opaque dot for the player's position
    let point = core::Point::new(mx, my);
    imgproc::circle(minimap, point, 4, color, -1, imgproc::LINE_8, 0)?;

    Ok(())
}

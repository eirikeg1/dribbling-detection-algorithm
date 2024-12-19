use crate::config::Config;
use crate::domain::data::models::{Annotation, BboxImage};
use opencv::core::{self, Mat, Rect, Scalar, Size};
use opencv::imgproc;
use opencv::prelude::*;

pub fn draw_annotations(
    frame: &mut Mat,
    annotations: &[Annotation],
    image_id: &String,
    config: &Config,
) -> opencv::Result<()> {
    let annotations: Vec<Annotation> = annotations
        .into_iter()
        .filter(|ann| ann.image_id == *image_id)
        .cloned()
        .collect();

    let scale_factor = config.visualization.scale_factor;
    // Scale the frame before extending
    // scale_frame(frame, scale_factor)?;

    // Draw bounding boxes on the main frame
    for annotation in annotations.iter() {
        if let Some(bbox_image) = &annotation.bbox_image {
            draw_bbox_image(frame, bbox_image, scale_factor, get_team_color(&annotation))?;
        }
    }

    let minimap_height = config.visualization.minimap_height;
    let minimap_width = config.visualization.minimap_width;

    // Extend the main frame to make space for the minimap
    let extended_height = frame.rows() + minimap_height;
    let extended_width = frame.cols().max(minimap_width);
    let mut extended_frame = Mat::zeros(extended_height, extended_width, frame.typ())?.to_mat()?;

    // Copy the original frame to the extended frame
    let roi_main = Rect::new(0, 0, frame.cols(), frame.rows());
    let mut extended_roi_main = Mat::roi_mut(&mut extended_frame, roi_main)?;
    frame.copy_to(&mut extended_roi_main)?;

    // Create the minimap
    let mut minimap = Mat::zeros(minimap_height, minimap_width, frame.typ())?.to_mat()?;
    imgproc::rectangle(
        &mut minimap,
        Rect::new(0, 0, minimap_width, minimap_height),
        Scalar::new(69.0, 160.0, 40.0, 255.0), // Green background
        -1,
        imgproc::LINE_8,
        0,
    )?;

    // Draw pitch points onto the minimap
    for annotation in annotations.iter() {
        if let Some(bbox_pitch) = &annotation.bbox_pitch {
            draw_pitch_point_on_minimap(
                &mut minimap,
                bbox_pitch.x_bottom_middle,
                bbox_pitch.y_bottom_middle,
                config,
                get_team_color(annotation),
            )?;
        }
    }

    // Copy the minimap to the extended frame below the main frame
    let minimap_x_offset = (frame.cols() - minimap_width) / 2; // Center horizontally
    let roi_minimap = Rect::new(
        minimap_x_offset,
        frame.rows(),
        minimap_width,
        minimap_height,
    );
    let mut extended_roi_minimap = Mat::roi_mut(&mut extended_frame, roi_minimap)?;
    minimap.copy_to(&mut extended_roi_minimap)?;

    // Replace the original frame with the extended frame
    *frame = extended_frame;

    Ok(())
}

fn get_team_color(annotation: &Annotation) -> Scalar {
    let team_id = &annotation.attributes.as_ref().unwrap().team;

    // Determine color based on team_id
    match team_id.as_deref() {
        Some("left") => Scalar::new(0.0, 0.0, 255.0, 255.0), // Blue for team A
        Some("right") => Scalar::new(0.0, 255.0, 0.0, 255.0), // Green for team B
        _ => Scalar::new(255.0, 0.0, 0.0, 255.0),            // Default: Red
    }
}

fn scale_frame(frame: &mut Mat, scale: f64) -> opencv::Result<()> {
    let new_size = Size {
        width: (frame.cols() as f64 * scale) as i32,
        height: (frame.rows() as f64 * scale) as i32,
    };
    let mut resized_frame = Mat::default();
    imgproc::resize(
        &*frame,
        &mut resized_frame,
        new_size,
        0.0,
        0.0,
        imgproc::INTER_LINEAR,
    )?;
    *frame = resized_frame;
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

    // Draw opaque circles onto the minimap
    let point = core::Point::new(mx, my);
    imgproc::circle(minimap, point, 4, color, -1, imgproc::LINE_8, 0)?;

    Ok(())
}
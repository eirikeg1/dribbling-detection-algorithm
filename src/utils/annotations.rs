use super::annotation_calculations::get_annotation_color;
use super::draw_pitch_minimap::draw_pitch_markings_on_minimap;
use crate::config::Config;
use crate::data::models::{Annotation, BboxImage};
use opencv::core::{self, Mat, Rect, Scalar};
use opencv::imgproc;
use opencv::prelude::*;
use std::collections::HashMap;

pub fn draw_annotations(
    frame: &mut Mat,
    annotations: &[Annotation],
    categories: &HashMap<String, u32>,
    image_id: &str,
    config: &Config,
    inner_rad: f64,
    outer_rad: f64,
) -> opencv::Result<()> {
    let annotations: Vec<Annotation> = annotations
        .iter()
        .filter(|ann| ann.image_id == *image_id)
        .cloned()
        .collect();

    let ball_id = categories.get("ball").unwrap_or(&4);
    let scale_factor = config.visualization.scale_factor;

    // Draw main 2D boxes
    for annotation in &annotations {
        if let Some(bbox_image) = &annotation.bbox_image {
            draw_bbox_image(
                frame,
                bbox_image,
                scale_factor,
                get_annotation_color(annotation, categories),
            )?;
        }
    }

    // Prepare extended frame and minimap
    let minimap_height = config.visualization.minimap_height;
    let minimap_width = config.visualization.minimap_width;
    let extended_height = frame.rows() + minimap_height;
    let extended_width = frame.cols().max(minimap_width);
    let mut extended_frame = Mat::zeros(extended_height, extended_width, frame.typ())?.to_mat()?;
    let roi_main = Rect::new(0, 0, frame.cols(), frame.rows());
    let mut extended_roi_main = Mat::roi_mut(&mut extended_frame, roi_main)?;
    frame.copy_to(&mut extended_roi_main)?;

    let mut minimap = Mat::zeros(minimap_height, minimap_width, frame.typ())?.to_mat()?;
    imgproc::rectangle(
        &mut minimap,
        Rect::new(0, 0, minimap_width, minimap_height),
        Scalar::new(69.0, 160.0, 40.0, 255.0),
        -1,
        imgproc::LINE_8,
        0,
    )?;
    draw_pitch_markings_on_minimap(&mut minimap, config)?;

    for annotation in &annotations {
        if annotation.category_id == *ball_id {
            if config.dribbling_detection.use_2d {
                // Draw circles in pitch space on the minimap
                if let Some(bbox_pitch) = &annotation.bbox_pitch {
                    let mx = ((bbox_pitch.x_bottom_middle - config.visualization.x_min)
                        / (config.visualization.x_max - config.visualization.x_min)
                        * minimap_width as f64) as i32;
                    let my = ((bbox_pitch.y_bottom_middle - config.visualization.y_min)
                        / (config.visualization.y_max - config.visualization.y_min)
                        * minimap_height as f64) as i32;

                    let rx_inner = (inner_rad
                        / (config.visualization.x_max - config.visualization.x_min)
                        * minimap_width as f64)
                        .min(
                            inner_rad / (config.visualization.y_max - config.visualization.y_min)
                                * minimap_height as f64,
                        ) as i32;
                    let rx_outer = (outer_rad
                        / (config.visualization.x_max - config.visualization.x_min)
                        * minimap_width as f64)
                        .min(
                            outer_rad / (config.visualization.y_max - config.visualization.y_min)
                                * minimap_height as f64,
                        ) as i32;

                    imgproc::circle(
                        &mut minimap,
                        core::Point::new(mx, my),
                        rx_outer,
                        Scalar::new(0.0, 242.0, 254.0, 154.0),
                        2,
                        imgproc::LINE_8,
                        0,
                    )?;
                    imgproc::circle(
                        &mut minimap,
                        core::Point::new(mx, my),
                        rx_inner,
                        Scalar::new(55.0, 166.0, 255.0, 0.0),
                        1,
                        imgproc::LINE_8,
                        0,
                    )?;
                }
            } else {
                // Draw circles on the main image based on bounding box center
                if let Some(bbox_image) = &annotation.bbox_image {
                    let cx = ((bbox_image.x + bbox_image.w / 2.0) * scale_factor) as i32;
                    let cy = ((bbox_image.y + bbox_image.h / 2.0) * scale_factor) as i32;

                    // Inner/outer circle sizes in pixels. Adjust as needed for your scale.
                    let inner_px = inner_rad as i32;
                    let outer_px = outer_rad as i32;

                    imgproc::circle(
                        &mut extended_roi_main, // draw onto main frame region
                        core::Point::new(cx, cy),
                        outer_px,
                        Scalar::new(0.0, 242.0, 254.0, 154.0),
                        1,
                        imgproc::LINE_8,
                        0,
                    )?;
                    imgproc::circle(
                        &mut extended_roi_main,
                        core::Point::new(cx, cy),
                        inner_px,
                        Scalar::new(55.0, 166.0, 255.0, 0.0),
                        1,
                        imgproc::LINE_8,
                        0,
                    )?;
                }
            }
        }

        // Draw basic positions for players/others on minimap if needed.
        if let Some(bbox_pitch) = &annotation.bbox_pitch {
            let color = get_annotation_color(annotation, categories);
            draw_pitch_point_on_minimap(
                &mut minimap,
                bbox_pitch.x_bottom_middle,
                bbox_pitch.y_bottom_middle,
                config,
                color,
            )?;
        }
    }

    let minimap_x_offset = (frame.cols() - minimap_width) / 2;
    let roi_minimap = Rect::new(
        minimap_x_offset,
        frame.rows(),
        minimap_width,
        minimap_height,
    );
    let mut extended_roi_minimap = Mat::roi_mut(&mut extended_frame, roi_minimap)?;
    minimap.copy_to(&mut extended_roi_minimap)?;
    *frame = extended_frame;
    Ok(())
}

// Basic bounding-box drawing in 2D
fn draw_bbox_image(
    frame: &mut Mat,
    bb: &BboxImage,
    scale: f64,
    color: Scalar,
) -> opencv::Result<()> {
    let x = (bb.x * scale) as i32;
    let y = (bb.y * scale) as i32;
    let w = (bb.w * scale) as i32;
    let h = (bb.h * scale) as i32;
    imgproc::rectangle(frame, Rect::new(x, y, w, h), color, 1, imgproc::LINE_8, 0)?;
    Ok(())
}

// fn draw_bbox_image(
//     frame: &mut Mat,
//     bbox_image: &BboxImage,
//     scale: f64,
//     color: Scalar,
// ) -> opencv::Result<()> {
//     let x = (bbox_image.x as f64 * scale) as i32;
//     let y = (bbox_image.y as f64 * scale) as i32;
//     let w = (bbox_image.w as f64 * scale) as i32;
//     let h = (bbox_image.h as f64 * scale) as i32;

//     let rect = Rect::new(x, y, w, h);
//     imgproc::rectangle(frame, rect, color, 1, imgproc::LINE_8, 0)?;
//     Ok(())
// }

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

    // Draw an opaque dot for the player's (or ball's) position
    let point = core::Point::new(mx, my);
    imgproc::circle(minimap, point, 5, color, -1, imgproc::LINE_8, 0)?;

    Ok(())
}

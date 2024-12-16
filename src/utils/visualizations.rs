use opencv::core::{Point, Scalar};
use opencv::highgui;
use opencv::imgcodecs;
use opencv::imgproc;
use opencv::prelude::*;
use opencv::videoio::{VideoCapture, CAP_ANY};
use std::fs;
use std::path::Path;

use crate::domain::models::Annotation;

/// Visualize annotations on a video file or a directory of image frames
pub fn visualize_video(video_path: &Path, annotations: &[Annotation]) -> opencv::Result<()> {
    if video_path.is_dir() {
        // Treat as a sequence of images (e.g., video_path/img1/*.jpg)
        let image_dir = video_path.join("img1");
        if !image_dir.is_dir() {
            eprintln!("Expected 'img1' directory inside {:?}", video_path);
            return Err(opencv::Error::new(
                opencv::core::StsError,
                "No 'img1' directory found",
            ));
        }

        let mut image_paths: Vec<_> = fs::read_dir(&image_dir)
            .map_err(|e| opencv::Error::new(opencv::core::StsError, format!("IO Error: {}", e)))?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|ext| ext.to_str()) == Some("jpg"))
            .collect();

        // Sort by filename to ensure the correct frame order
        image_paths.sort();

        for image_path in image_paths {
            let mut frame =
                imgcodecs::imread(image_path.to_str().unwrap(), imgcodecs::IMREAD_COLOR)?;
            if frame.empty() {
                continue;
            }

            draw_annotations(&mut frame, annotations)?;

            highgui::imshow("Image Sequence Visualization", &frame)?;
            // Break loop on 'q' key press
            if highgui::wait_key(30)? == 113 {
                break;
            }
        }

        Ok(())
    } else {
        // Treat as a video file
        let mut capture = VideoCapture::from_file(video_path.to_str().unwrap(), CAP_ANY)?;
        if !capture.is_opened()? {
            eprintln!("Unable to open video: {:?}", video_path);
            return Err(opencv::Error::new(
                opencv::core::StsError,
                "Unable to open video",
            ));
        }

        let mut frame = opencv::core::Mat::default();
        while capture.read(&mut frame)? {
            if frame.empty() {
                break;
            }

            draw_annotations(&mut frame, annotations)?;

            highgui::imshow("Video Visualization", &frame)?;
            // Break loop on 'q' key press
            if highgui::wait_key(10)? == 113 {
                break;
            }
        }

        Ok(())
    }
}

/// Draw bounding box annotations onto the given frame
fn draw_annotations(
    frame: &mut opencv::core::Mat,
    annotations: &[Annotation],
) -> opencv::Result<()> {
    for annotation in annotations {
        if let Some(bbox) = &annotation.bbox_image {
            if let Some(array) = bbox.as_array() {
                if array.len() == 4 {
                    let x = array[0].as_f64().unwrap_or(0.0) as i32;
                    let y = array[1].as_f64().unwrap_or(0.0) as i32;
                    let width = array[2].as_f64().unwrap_or(0.0) as i32;
                    let height = array[3].as_f64().unwrap_or(0.0) as i32;

                    let rect = opencv::core::Rect::new(x, y, width, height);
                    imgproc::rectangle(
                        frame,
                        rect,
                        Scalar::new(0.0, 0.0, 255.0, 0.0),
                        2,
                        imgproc::LINE_8,
                        0,
                    )?;
                }
            }
        }
    }
    Ok(())
}

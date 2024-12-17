use opencv::highgui;
use opencv::imgcodecs;
use opencv::prelude::*;
use opencv::videoio::VideoWriter;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use crate::domain::data::models::Image;
use crate::domain::data::models::{Annotation, BboxImage, BboxPitch};
use opencv::core::{self, Rect, Scalar, Size, Mat};
use opencv::{imgproc, prelude::*};

// Constants for minimap size and position
const MINIMAP_X: i32 = 10;
const MINIMAP_Y: i32 = 10;
const MINIMAP_WIDTH: i32 = 200;
const MINIMAP_HEIGHT: i32 = 100;

// Pitch coordinate ranges for transforming pitch coords onto minimap
// Adjust these as necessary for your coordinate system:
const X_MIN: f64 = -60.0;
const X_MAX: f64 = 60.0;
const Y_MIN: f64 = -34.0;
const Y_MAX: f64 = 34.0;

pub fn visualize_or_store_video(
    image_dir: &Path,
    annotations: &[Annotation],
    images: &[Image],
    mode: &str,
    output_path: &str,
    file_name: &str
) -> opencv::Result<()> {

    let image_map: HashMap<String, String> = images
        .iter()
        .map(|image| (image.file_name.clone(), image.image_id.clone()))
        .collect();

    if !image_dir.is_dir() {
        eprintln!("Expected directory but found none at {:?}", image_dir);
        return Err(opencv::Error::new(
            opencv::core::StsError,
            "Directory not found",
        ));
    }

    // Create the output directory if it does not exist
    let output_dir_path = Path::new(output_path);
    if !output_dir_path.exists() {
        fs::create_dir_all(output_dir_path)
            .map_err(|e| opencv::Error::new(opencv::core::StsError, format!("Failed to create output directory: {}", e)))?;
    }

    let mut image_paths: Vec<_> = fs::read_dir(image_dir)
        .map_err(|e| opencv::Error::new(opencv::core::StsError, format!("IO Error: {}", e)))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|ext| ext.to_str()) == Some("jpg"))
        .collect();

    let video_path = output_dir_path.join(format!("{}.avi", file_name));
    image_paths.sort();

    let video_name = image_dir.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("output");

    let mut writer: Option<VideoWriter> = None;
    let mut frame_count = 0;

    // println!("Found {} images in directory: {}", image_paths.len(), image_dir.display());
    for (image_num, image_path) in image_paths.into_iter().enumerate() {
        let mut frame = imgcodecs::imread(image_path.to_str().unwrap(), imgcodecs::IMREAD_COLOR)?;
        if frame.empty() {
            eprintln!("Warning: Empty frame for image path {:?}", image_path);
            continue;
        }

        if writer.is_none() {
            writer = Some(initialize_writer(&video_path, &frame)?);
            eprintln!("VideoWriter initialized for file: {}", video_path.display());
        }
        let image_file_name = image_path
            .to_string_lossy()
            .split('/')
            .last()
            .unwrap_or("")
            .to_string();
        
        let image_id = image_map.get(&image_file_name).unwrap_or(&image_file_name);

        draw_annotations(&mut frame, annotations, image_id)?;

        if mode == "download" {
            process_download_mode(&mut writer, &frame)?;
        } else {
            process_visualization_mode(&frame)?;
        }

        frame_count += 1;
    }
    
    if let Some(ref mut writer) = writer {
        writer.release()?;
        eprintln!("VideoWriter released for file: {} with {} frames", video_path.display(), frame_count);
    }
    
    Ok(())
}


fn initialize_writer(video_path: &Path, frame: &opencv::core::Mat) -> opencv::Result<VideoWriter> {
    let frame_size = frame.size()?;
    if frame_size.width > 0 && frame_size.height > 0 {
        let writer = VideoWriter::new(
            video_path.to_str().unwrap(),
            opencv::videoio::VideoWriter::fourcc('m', 'p', '4', 'v')?,
            30.0,
            frame_size,
            true,
        )?;
        if !writer.is_opened()? {
            eprintln!("VideoWriter failed to open for path: {}", video_path.display());
        }
        Ok(writer)
    } else {
        Err(opencv::Error::new(
            opencv::core::StsError,
            format!("Frame size is zero, skipping writer initialization for path: {}", video_path.display()),
        ))
    }
}

fn process_download_mode(writer: &mut Option<VideoWriter>, frame: &opencv::core::Mat) -> opencv::Result<()> {
    if let Some(ref mut writer) = writer {
        writer.write(frame).map_err(|e| {
            eprintln!("Failed to write frame: {:?}", e);
            e
        })?;
    } else {
        eprintln!("VideoWriter is not initialized for current video.");
    }
    Ok(())
}

fn process_visualization_mode(frame: &opencv::core::Mat) -> opencv::Result<()> {
    highgui::imshow("Image Sequence Visualization", frame)?;
    if highgui::wait_key(30)? == 113 { // Break loop on 'q' key press
        return Ok(());
    }
    Ok(())
}

pub fn draw_annotations(
    frame: &mut Mat,
    annotations: &[Annotation],
    image_id: &String,
) -> opencv::Result<()> {

    let annotations: Vec<Annotation> = annotations.into_iter()
        .filter(|ann| ann.image_id == image_id.to_string())
        .cloned()
        .collect();


    let annotations: &[Annotation] = &annotations;

    // Draw bounding boxes on the main frame
    for annotation in annotations.into_iter() {
        if let Some(bbox_image) = &annotation.bbox_image {
            draw_bbox_image(frame, bbox_image)?;
        }
    }

    // Create the minimap
    let mut minimap = Mat::zeros(MINIMAP_HEIGHT, MINIMAP_WIDTH, frame.typ())?.to_mat()?;
    // Fill minimap with gray color
    imgproc::rectangle(
        &mut minimap,
        Rect::new(0, 0, MINIMAP_WIDTH, MINIMAP_HEIGHT),
        Scalar::new(128.0, 128.0, 128.0, 0.0),
        -1,
        imgproc::LINE_8,
        0,
    )?;

    // Draw pitch coordinates onto the minimap
    for annotation in annotations {
        if let Some(bbox_pitch) = &annotation.bbox_pitch {
            draw_pitch_point_on_minimap(&mut minimap, bbox_pitch.x_bottom_middle, bbox_pitch.y_bottom_middle)?;
        }
    }

    // Now create an overlay the size of the entire frame and place the minimap in it
    let mut overlay = Mat::zeros(frame.rows(), frame.cols(), frame.typ())?.to_mat()?;
    let rows = minimap.rows();
    let cols = minimap.cols();
    for r in 0..rows {
        for c in 0..cols {
            let pixel = minimap.at_2d::<core::Vec3b>(r, c)?;
            *overlay.at_2d_mut::<core::Vec3b>(MINIMAP_Y + r, MINIMAP_X + c)? = *pixel;
        }
    }

    // Blend the overlay with the main frame
    let mut frame_clone = frame.clone();
    core::add_weighted(&frame_clone, 1.0, &overlay, 0.3, 0.0, frame, -1)?;

    Ok(())
}

fn draw_bbox_image(frame: &mut Mat, bbox_image: &BboxImage) -> opencv::Result<()> {
    let x = bbox_image.x as i32;
    let y = bbox_image.y as i32;
    let w = bbox_image.w as i32;
    let h = bbox_image.h as i32;

    let rect = Rect::new(x, y, w, h);
    imgproc::rectangle(
        frame,
        rect,
        Scalar::new(0.0, 0.0, 255.0, 0.0), // Red
        2,
        imgproc::LINE_8,
        0,
    )?;
    Ok(())
}

fn draw_pitch_point_on_minimap(minimap: &mut Mat, pitch_x: f64, pitch_y: f64) -> opencv::Result<()> {
    // Convert pitch coordinates to minimap coordinates
    let mx = ((pitch_x - X_MIN) / (X_MAX - X_MIN)) * (MINIMAP_WIDTH as f64);
    let my = ((pitch_y - Y_MIN) / (Y_MAX - Y_MIN)) * (MINIMAP_HEIGHT as f64);

    // Draw a white circle at the transformed point
    let point = core::Point::new(mx as i32, my as i32);
    imgproc::circle(
        minimap,
        point,
        3,
        Scalar::new(255.0, 255.0, 255.0, 0.0), // White
        -1,
        imgproc::LINE_8,
        0,
    )?;

    Ok(())
}

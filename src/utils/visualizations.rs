use opencv::highgui;
use opencv::imgcodecs;
use opencv::prelude::*;
use opencv::videoio::VideoWriter;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use crate::config::Config;
use crate::domain::data::models::Image;
use crate::domain::data::models::{Annotation, BboxImage, BboxPitch};
use opencv::core::{self, Rect, Scalar, Size, Mat};
use opencv::{imgproc, prelude::*};

use super::annotations::draw_annotations;

pub fn visualize_or_store_video(
    image_dir: &Path,
    annotations: &[Annotation],
    images: &[Image],
    mode: &str,
    output_path: &str,
    file_name: &str,
    config: &Config
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

        draw_annotations(&mut frame, annotations, image_id, config)?;

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
            20.0,
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



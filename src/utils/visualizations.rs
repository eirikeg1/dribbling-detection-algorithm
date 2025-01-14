use crate::domain::data::models::Annotation;
use crate::{config::Config, domain::events::drible_models::DribbleEvent};
use opencv::{core::Mat, highgui, prelude::*, videoio::VideoWriter};
use std::collections::HashMap;
use std::{
    fs,
    path::{Path, PathBuf},
};

use super::{annotations::draw_annotations, image_calculations::scale_frame};

/// A builder to handle video creation or visualization,
/// allowing you to add frames, one at a time.
pub struct VisualizationBuilder<'a> {
    mode: &'a str,
    output_path: PathBuf,
    config: &'a Config,
    writer: Option<VideoWriter>,
    frame_count: usize,
}

impl<'a> VisualizationBuilder<'a> {
    pub fn new(mode: &'a str, file_name: &'a str, config: &'a Config) -> opencv::Result<Self> {
        let output_dir_path = Path::new(&config.data.output_path);

        if !output_dir_path.exists() {
            fs::create_dir_all(output_dir_path).map_err(|e| {
                opencv::Error::new(
                    opencv::core::StsError,
                    format!("Failed to create output directory: {}", e),
                )
            })?;
        }

        let output_path = output_dir_path.join(format!("{}.avi", file_name));

        println!(
            "\nFile name: {file_name}, Output path: {}",
            output_path.display()
        );
        Ok(Self {
            mode,
            output_path: output_path.to_path_buf(),
            config,
            writer: None,
            frame_count: 0,
        })
    }

    /// Add **one** frame
    ///
    /// - If `writer` is not initialized yet, it will initialize based on the
    ///   current frame’s dimensions.
    /// - Scales the frame and draws annotations based on config.
    /// - Then it either writes the frame (`download` mode) or displays it
    ///   (`visualization` mode).
    pub fn add_frame(
        &mut self,
        frame: &mut Mat,
        image_id: Option<&str>,
        annotations: Option<&[Annotation]>,
        categories: &HashMap<String, u32>,
        drible_event: Option<DribbleEvent>,
    ) -> opencv::Result<()> {
        if frame.empty() {
            eprintln!("Warning: Empty frame was provided.");
            return Ok(());
        }

        scale_frame(frame, self.config)?;

        if let (Some(id), Some(ann)) = (image_id, annotations) {
            draw_annotations(frame, ann, categories, drible_event, &id, self.config)?;
        }

        if self.writer.is_none() {
            self.writer = Some(
                initialize_writer(&self.output_path, frame).expect("failed to initialize writer"),
            );
        }

        if self.mode == "download" {
            if let Some(ref mut writer) = self.writer {
                writer.write(frame).expect("failed to write to video");
            } else {
                eprintln!("VideoWriter is not initialized—cannot write frame.");
            }
        } else {
            highgui::imshow("Image Sequence Visualization", frame).expect("failed to show frame");
        }

        self.frame_count += 1;
        Ok(())
    }

    pub fn finish(&mut self) -> opencv::Result<()> {
        if let Some(ref mut writer) = self.writer {
            writer.release().expect("failed to release writer");
            // eprintln!(
            //     "Saved {} frames to '{}'.",
            //     self.frame_count,
            //     self.output_path.display(),
            // );
        } else {
            eprintln!("No VideoWriter was created. Nothing to finalize.");
        }

        Ok(())
    }
}

fn initialize_writer(video_path: &Path, frame: &opencv::core::Mat) -> opencv::Result<VideoWriter> {
    let frame_size = frame.size()?;

    if frame_size.width > 0 && frame_size.height > 0 {
        // println!("Initializing writer for path: {}", video_path.display());
        let writer = VideoWriter::new(
            video_path.to_str().unwrap(),
            // Use a codec that is more widely supported, e.g., 'MJPG'
            opencv::videoio::VideoWriter::fourcc('M', 'J', 'P', 'G')?,
            20.0,
            frame_size,
            true,
        )?;
        if !writer
            .is_opened()
            .expect("failed to check if writer is opened")
        {
            eprintln!(
                "VideoWriter failed to open for path: {}",
                video_path.display()
            );
        }
        Ok(writer)
    } else {
        Err(opencv::Error::new(
            opencv::core::StsError,
            format!(
                "Frame size is zero, skipping writer initialization for path: {}",
                video_path.display()
            ),
        ))
    }
}

/// Wait for user input to continue or quit. Returns false if user inputs "q".
/// If `autoplay` is enabled in the config, it will return true immediately.
pub fn handle_keyboard_input(config: &Config) -> Result<bool, opencv::Error> {
    if config.visualization.autoplay {
        if highgui::wait_key(1)? == 113 {
            return Ok(false);
        }

        return Ok(true);
    }

    if highgui::wait_key(0)? == 113 {
        return Ok(false);
    }

    Ok(true)
}

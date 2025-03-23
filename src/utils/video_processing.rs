use chrono::Utc;
use serde_json::to_writer_pretty;
use std::fs::{create_dir_all, copy, File};
use std::io::Result;
use std::path::Path;

use crate::data::models::{ReviewedVideoData, VideoData};

pub fn export_reviewed_data(
    config_output: &Path,
    all_reviewed_video_data: &[ReviewedVideoData],
) -> Result<()> {
    let now = Utc::now().format("%y-%m-%d_%H-%M-%S").to_string();

    let dribbles_folder = config_output.join(format!("dribbles-{}", now));
    let tackles_folder = config_output.join(format!("tackles-{}", now));
    let none_folder = config_output.join(format!("none-{}", now));

    create_dir_all(&dribbles_folder)?;
    create_dir_all(&tackles_folder)?;
    create_dir_all(&none_folder)?;

    for (idx, reviewed) in all_reviewed_video_data.iter().enumerate() {
        // Export dribble data
        for (vid_idx, video_data) in reviewed.dribble_data.iter().enumerate() {
            let video_folder = dribbles_folder.join(format!("video_{}_{}", idx, vid_idx));
            store_video_data(&video_folder, video_data)?;
        }

        // Export tackle data
        for (vid_idx, video_data) in reviewed.tackle_data.iter().enumerate() {
            let video_folder = tackles_folder.join(format!("video_{}_{}", idx, vid_idx));
            store_video_data(&video_folder, video_data)?;
        }

        // Export other/none data
        for (vid_idx, video_data) in reviewed.other_data.iter().enumerate() {
            let video_folder = none_folder.join(format!("video_{}_{}", idx, vid_idx));
            store_video_data(&video_folder, video_data)?;
        }
    }
    Ok(())
}

fn store_video_data(folder: &Path, video_data: &VideoData) -> Result<()> {
    create_dir_all(folder)?;
    let img_folder = folder.join("img1");
    create_dir_all(&img_folder)?;

    for image_path in &video_data.image_paths {
        if let Some(name) = image_path.file_name() {
            copy(image_path, img_folder.join(name))?;
        }
    }

    let labels_file = folder.join("Labels-GameState.json");
    let file = File::create(labels_file)?;
    to_writer_pretty(file, &video_data.labels)?;
    Ok(())
}

use dribbling_detection_algorithm::data::download_data::download_and_extract_dataset;
use dribbling_detection_algorithm::data::models::{Annotation, VideoData};
use dribbling_detection_algorithm::dribbling_detection::create_dribble_models::{
    get_ball_model, get_player_models,
};
use dribbling_detection_algorithm::dribbling_detection::dribble_detector::{self, DribbleDetector};
use dribbling_detection_algorithm::dribbling_detection::dribble_models::{
    Ball, DribbleEvent, DribbleFrame,
};
use dribbling_detection_algorithm::utils::annotation_calculations::filter_annotations;
use dribbling_detection_algorithm::utils::keyboard_input::{
    wait_for_keyboard_input, KeyboardInput,
};
use dribbling_detection_algorithm::utils::visualizations::VisualizationBuilder;
use dribbling_detection_algorithm::{config::Config, data::dataset::Dataset};
use opencv::core::MatTraitConst;
use opencv::imgcodecs;
use std::collections::HashMap;
use std::env;
use std::fs;
use tokio::io;
use tokio::runtime::Runtime;

fn main() {
    // Check for command-line arguments
    let args: Vec<String> = env::args().collect();
    let should_download = args.contains(&"--download".to_string());

    if should_download {
        println!("Data download initiated...");
        let config_content =
            fs::read_to_string("config.toml").expect("Unable to read the config file");
        let config: Config =
            toml::from_str(&config_content).expect("Unable to parse the config file");

        // Use Tokio runtime to run async code
        let rt = Runtime::new().unwrap();
        rt.block_on(download_and_extract_dataset(&config));
        println!("Data download complete.");
    }

    println!("Continuing with regular execution...");

    // Load the configuration file
    let config_content = fs::read_to_string("config.toml").expect("Unable to read the config file");
    let config: Config = toml::from_str(&config_content).expect("Unable to parse the config file");

    // Example: Print loaded configuration
    println!("{:#?}", config);

    let video_mode: &String = &config.general.video_mode;

    let dataset = Dataset::new(config.clone());
    let data_iter = dataset.iter_subset(&"interpolated-predictions");
    let inner_rad = config.dribbling_detection.inner_radius;
    let outer_rad = config.dribbling_detection.outer_radius;
    let dribble_detector = DribbleDetector::new(
        inner_rad,
        outer_rad,
        config.dribbling_detection.inner_threshold,
        config.dribbling_detection.outer_threshold,
        config.dribbling_detection.outer_in_threshold,
        config.dribbling_detection.outer_out_threshold,
    );
    process_videos(data_iter, config.clone(), video_mode, dribble_detector);
}

fn process_videos(
    data_iter: impl Iterator<Item = io::Result<VideoData>>,
    config: Config,
    video_mode: &String,
    mut dribble_detector: DribbleDetector,
) {
    let mut all_detected_events: HashMap<String, Vec<DribbleEvent>> = HashMap::new();

    // Iterate over videos
    for (vid_num, video_data) in data_iter.enumerate() {
        let video_data = video_data.unwrap();
        let image_map: HashMap<String, String> = video_data
            .labels
            .images
            .iter()
            .map(|image| (image.file_name.clone(), image.image_id.clone()))
            .collect();

        let category_map: HashMap<String, u32> = video_data
            .labels
            .categories
            .iter()
            .map(|c| (c.name.clone(), c.id))
            .collect();

        let annotations: Vec<Annotation> = video_data.labels.annotations.clone();
        let file_name = format!("video_{}", vid_num);

        let mut visualization_builder =
            VisualizationBuilder::new(video_mode.as_str(), &file_name, &config)
                .expect("Failed to create visualization builder");

        let mut detected_events: Vec<DribbleEvent> = Vec::new();

        for (frame_num, image_path) in video_data.image_paths.into_iter().enumerate() {
            let image_file_name = image_path
                .to_string_lossy()
                .split('/')
                .last()
                .unwrap_or("")
                .to_string();

            let image_id = image_map.get(&image_file_name).unwrap_or(&image_file_name);

            let mut frame =
                imgcodecs::imread(image_path.to_str().unwrap(), imgcodecs::IMREAD_COLOR).unwrap();

            let filtered_annotations = filter_annotations(
                image_id,
                annotations.clone(),
                &category_map,
                config.dribbling_detection.ignore_person_classes,
                config.dribbling_detection.ignore_teams,
            );
            let ball_model = get_ball_model(&category_map, &filtered_annotations);
            let player_models = get_player_models(&category_map, &filtered_annotations);

            if player_models.is_none() {
                println!("(In main): No players found in frame. Skipping frame...");
                continue;
            }
            // If no ball is found, you might decide to skip or keep going:
            // if ball_model.is_none() { ... }

            let dribble_frame = DribbleFrame {
                frame_number: frame_num as u32,
                players: player_models.unwrap(),
                ball: ball_model.unwrap_or(Ball { x: 0.0, y: 0.0 }),
            };

            // The detector may return an event that just *finished* on this frame
            let maybe_event = dribble_detector.process_frame(dribble_frame);

            if let Some(dribble_event) = maybe_event.clone() {
                if dribble_event.detected_dribble || dribble_event.detected_tackle {
                    println!("Detected dribble event: {:?}", dribble_event.frames);
                    detected_events.push(dribble_event);
                }
            }

            visualization_builder
                .add_frame(
                    &mut frame,
                    Some(image_id),
                    Some(&filtered_annotations),
                    &category_map,
                    maybe_event,
                )
                .expect("Failed to add frame");

            // Handle keyboard input
            let input_value =
                wait_for_keyboard_input(&config).expect("There was an error with keyboard input");

            match input_value {
                KeyboardInput::Quit => {
                    visualization_builder
                        .finish()
                        .expect("Failed to finish visualization");
                    return;
                }
                KeyboardInput::NextFrame => {}
                KeyboardInput::PreviousFrame => {}
                KeyboardInput::NextVideo => {
                    visualization_builder
                        .finish()
                        .expect("Failed to finish visualization");
                    break;
                }
            }
        }

        all_detected_events.insert(file_name.clone(), detected_events.clone());

        // After processing all frames of the video, merge consecutive dribble events
        let merged_events = combine_consecutive_events(detected_events);

        visualization_builder
            .finish()
            .expect("Failed to finish visualization");

        if config.general.log_level == "debug" && !merged_events.is_empty() {
            println!("\n\nDetected dribble events for video {}:", file_name);
            for ev in merged_events {
                // You can customize printing logic (tackle vs dribble, etc.)
                if ev.detected_tackle {
                    println!(" * Tackle event detected: {:?}", ev.frames);
                } else {
                    println!(" * Dribble event detected: {:?}", ev.frames);
                }
            }
        }
    }
}

/// Merges consecutive dribble events if the start of one event
/// is immediately after (or the same as) the end of the previous event.
fn combine_consecutive_events(mut events: Vec<DribbleEvent>) -> Vec<DribbleEvent> {
    events.sort_by_key(|e| e.start_frame);

    let mut merged: Vec<DribbleEvent> = Vec::new();
    for event in events {
        if let Some(last) = merged.last_mut() {
            if let Some(last_end) = last.end_frame {

                let same_type = (last.detected_tackle && event.detected_tackle)
                    || (last.detected_dribble && event.detected_dribble);

                if (event.start_frame <= last_end + 1) && same_type {
                    last.extend(&event);

                    if let Some(end) = event.end_frame {
                        last.end_frame = Some(end);
                    }
                    // If the new event is a tackle or a contested dribble, keep that info
                    last.detected_tackle |= event.detected_tackle;
                    last.detected_dribble |= event.detected_dribble;
                    last.ever_contested |= event.ever_contested;
                    continue;
                }
            }
        }
        // If not mergeable, push as a new event
        merged.push(event);
    }
    merged
}

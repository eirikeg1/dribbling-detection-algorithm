use chrono::Utc;
use dribbling_detection_algorithm::data::download_data::download_and_extract_dataset;
use dribbling_detection_algorithm::data::models::{
    Annotation, DribbleEventsExport, ExportInfo, VideoData, VideoDribbleEvents,
};
use dribbling_detection_algorithm::dribbling_detection::create_dribble_models::{
    get_ball_model, get_player_models,
};
use dribbling_detection_algorithm::dribbling_detection::dribble_detector::DribbleDetector;
use dribbling_detection_algorithm::dribbling_detection::dribble_models::{
    Ball, DribbleEvent, DribbleFrame,
};
use dribbling_detection_algorithm::utils::annotation_calculations::filter_annotations;
use dribbling_detection_algorithm::utils::keyboard_input::{
    wait_for_keyboard_input, KeyboardInput,
};
use dribbling_detection_algorithm::utils::visualizations::VisualizationBuilder;
use dribbling_detection_algorithm::{config::Config, data::dataset::Dataset};
use opencv::imgcodecs;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

static EXIT_FLAG: AtomicBool = AtomicBool::new(false);

fn main() {
    let start_time = Utc::now();
    let args: Vec<String> = env::args().collect();
    let should_download = args.contains(&"--download".to_string());

    if should_download {
        println!("Data download initiated...");
        let config_content =
            fs::read_to_string("config.toml").expect("Unable to read the config file");
        let config: Config =
            toml::from_str(&config_content).expect("Unable to parse the config file");

        let rt = Runtime::new().unwrap();
        rt.block_on(download_and_extract_dataset(&config));
        println!("Data download complete.");
    }

    println!("Continuing with regular execution...");

    let config_content = fs::read_to_string("config.toml").expect("Unable to read the config file");
    let config: Config = toml::from_str(&config_content).expect("Unable to parse the config file");

    println!("{:#?}", config);

    let video_mode: &String = &config.general.video_mode;
    let num_threads: usize = if config.general.video_mode == "display" {
        println!("Using 1 core since video mode is set to \"display\"");
        1
    } else {
        config.general.num_cores as usize
    };

    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .unwrap();

    let dataset = Dataset::new(config.clone());
    let data_iter: Vec<_> = dataset.iter_subset(&"interpolated-predictions").collect();
    let inner_rad = config.dribbling_detection.inner_radius;
    let outer_rad = config.dribbling_detection.outer_radius;

    // Shared map of all detected events
    let all_detected_events = Arc::new(Mutex::new(HashMap::new()));

    if num_threads > 1 {
        data_iter
            .par_iter()
            .enumerate()
            .for_each(|(vid_num, video_data)| {
                let video_name = format!("{:06}.jpg", vid_num);
                let dribble_detector = DribbleDetector::new(
                    video_name,
                    inner_rad,
                    outer_rad,
                    config.dribbling_detection.inner_threshold,
                    config.dribbling_detection.outer_threshold,
                    config.dribbling_detection.outer_in_threshold,
                    config.dribbling_detection.outer_out_threshold,
                );
                if EXIT_FLAG.load(Ordering::Relaxed) {
                    return;
                }
                let video_data = match video_data {
                    Ok(vd) => vd.clone(),
                    Err(_) => return,
                };

                // Each thread processes its video fully, collecting events
                let (file_name, merged_events) = process_video(
                    vid_num,
                    video_data,
                    config.clone(),
                    video_mode,
                    dribble_detector.clone(),
                );

                // Then each thread adds its entire collection at once
                let mut all_events = all_detected_events.lock().unwrap();
                all_events.insert(file_name, merged_events);
            });
    } else {
        data_iter
            .iter()
            .enumerate()
            .for_each(|(vid_num, video_data)| {
                let video_name = format!("{:06}.jpg", vid_num);
                let dribble_detector = DribbleDetector::new(
                    video_name,
                    inner_rad,
                    outer_rad,
                    config.dribbling_detection.inner_threshold,
                    config.dribbling_detection.outer_threshold,
                    config.dribbling_detection.outer_in_threshold,
                    config.dribbling_detection.outer_out_threshold,
                );
                if EXIT_FLAG.load(Ordering::Relaxed) {
                    return;
                }
                let video_data = match video_data {
                    Ok(vd) => vd.clone(),
                    Err(_) => return,
                };

                let (file_name, merged_events) = process_video(
                    vid_num,
                    video_data,
                    config.clone(),
                    video_mode,
                    dribble_detector.clone(),
                );

                let mut all_events = all_detected_events.lock().unwrap();
                all_events.insert(file_name, merged_events);
            });
    }

    // Once all threads finish, we can safely unwrap the final events
    let all_detected_events = Arc::try_unwrap(all_detected_events)
        .unwrap()
        .into_inner()
        .unwrap();

    // Assume `all_detected_events` is a HashMap<String, Vec<DribbleEvent>>
    // that you built during video processing.
    let export = DribbleEventsExport {
        info: ExportInfo {
            version: "dribble_events_1.0".to_string(),
            generated_at: Utc::now().to_rfc3339(),
        },
        videos: all_detected_events
            .iter()
            .enumerate()
            .map(|(i, (video_id, events))| VideoDribbleEvents {
                video_id: video_id.clone(),
                file_name: format!("{:06}.jpg", i + 1),
                dribble_events: events.clone().iter().map(|e| e.into()).collect(),
            })
            .collect(),
    };

    let json_data =
        serde_json::to_string_pretty(&export).expect("Error serializing dribble events to JSON");

    let json_path = Path::new(&config.data.output_path).join("dribble_events.json");
    fs::write(json_path, json_data).expect("Error writing dribble_events.json file");

    if config.general.log_level == "debug" {
        println!("\n\nFinal detected dribble events:");
        for (video, events) in &all_detected_events {
            println!("Video: {}", video);
            for event in events {
                if event.detected_tackle {
                    println!(" * Tackle event detected: {:?}", event.frames);
                } else {
                    println!(" * Dribble event detected: {:?}", event.frames);
                }
            }
        }
    }

    let cur_time = Utc::now();
    let duration = cur_time - start_time;

    println!(
        "\n\nDetected {} dribble events in {}H:{}M:{}S",
        all_detected_events.values().flatten().count(),
        duration.num_hours(),
        duration.num_minutes() % 60,
        duration.num_seconds() % 60
    );
}

/// Processes a single video and returns its name plus the merged dribble events.
fn process_video(
    vid_num: usize,
    video_data: VideoData,
    config: Config,
    video_mode: &String,
    mut dribble_detector: DribbleDetector,
) -> (String, Vec<DribbleEvent>) {
    if config.general.log_level == "debug" {
        println!("Processing video {}", vid_num);
    }

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
        if EXIT_FLAG.load(Ordering::Relaxed) {
            break;
        }
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

        let dribble_frame = DribbleFrame {
            frame_number: frame_num as u32,
            players: player_models.unwrap(),
            ball: ball_model.unwrap_or(Ball { x: 0.0, y: 0.0 }),
        };

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

        if config.general.video_mode != "display" {
            continue;
        }

        let input_value =
            wait_for_keyboard_input(&config).expect("There was an error with keyboard input");

        match input_value {
            KeyboardInput::Quit => {
                EXIT_FLAG.store(true, Ordering::Relaxed);
                visualization_builder
                    .finish()
                    .expect("Failed to finish visualization");
                break;
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

    let merged_events = combine_consecutive_events(detected_events);

    visualization_builder
        .finish()
        .expect("Failed to finish visualization");

    (file_name, merged_events)
}

/// Merges consecutive dribble events if the start of one event
/// is immediately after (or the same as) the end of the previous event,
/// and both events are of the same type (dribble or tackle).
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

use chrono::Utc;
use dribbling_detection_algorithm::data::dataset::load_dribble_events_map;
use dribbling_detection_algorithm::data::download_data::download_and_extract_dataset;
use dribbling_detection_algorithm::data::models::{
    Annotation, DribbleEventsExport, DribbleLabel, ExportInfo, VideoData, VideoDribbleEvents,
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

    println!("\nRunning dribbling detection");

    let config_content = fs::read_to_string("config.toml").expect("Unable to read the config file");
    let config: Config = toml::from_str(&config_content).expect("Unable to parse the config file");
    let config = config.apply_env_overrides();

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

    println!("Number of videos to process: {}", data_iter.len());

    // Shared map of all detected events
    let all_detected_events = Arc::new(Mutex::new(HashMap::new()));

    let dribble_events_map = if config.general.review_mode.unwrap_or(false) {
        load_dribble_events_map(&config)
    } else {
        None
    };

    let review_mode = config.general.review_mode.unwrap_or(false);

    // ---------------------------------------------------------------------------------------------
    // Data structures to store the reviewed data, if in review mode
    // ---------------------------------------------------------------------------------------------

    let mut reviewed_dribbles = if review_mode {
        Some(VideoData::default())
    } else {
        None
    };

    let mut reviewed_tackles = if review_mode {
        Some(VideoData::default())
    } else {
        None
    };

    let mut reviewed_other = if review_mode {
        Some(VideoData::default())
    } else {
        None
    };

    // ---------------------------------------------------------------------------------------------
    // Define the per-video processing as a closure to avoid duplication
    // ---------------------------------------------------------------------------------------------
    let process_item = |(vid_num, video_data): (usize, &Result<VideoData, _>)| {
        // Build a DribbleDetector for this video
        let video_name = format!("{:06}.jpg", vid_num);
        let dribble_detector = DribbleDetector::new(
            video_name.clone(),
            inner_rad,
            outer_rad,
            config.dribbling_detection.inner_threshold,
            config.dribbling_detection.outer_threshold,
            config.dribbling_detection.outer_in_threshold,
            config.dribbling_detection.outer_out_threshold,
        );

        // Check for early exit
        if EXIT_FLAG.load(Ordering::Relaxed) {
            return;
        }

        // Skip this video if it can't be unwrapped
        let video_data = match video_data {
            Ok(vd) => vd.clone(),
            Err(_) => return,
        };

        // Process the video
        let processed_video = process_video(
            vid_num,
            video_name,
            video_data,
            config.clone(),
            video_mode,
            dribble_detector.clone(),
            &dribble_events_map,
        );

        if processed_video.is_some() {
            let (file_name, merged_events) = processed_video.unwrap();
            // Then each worker (thread or single) adds all events to the global map
            let mut all_events = all_detected_events.lock().unwrap();
            all_events.insert(file_name, merged_events);
        }
    };

    // -------------------------------
    // Use parallel or sequential iteration based on num_threads
    // -------------------------------
    if num_threads > 1 {
        data_iter.par_iter().enumerate().for_each(process_item);
    } else {
        data_iter.iter().enumerate().for_each(process_item);
    }

    if config.general.review_mode.unwrap_or(false) {
        let cur_time = Utc::now();
        let duration = cur_time - start_time;

        println!(
            "\n\nReview mode done in {}H:{}M:{}S",
            duration.num_hours(),
            duration.num_minutes() % 60,
            duration.num_seconds() % 60
        );
        return;
    }

    // Once all threads finish, safely unwrap the final events
    let all_detected_events = Arc::try_unwrap(all_detected_events)
        .unwrap()
        .into_inner()
        .unwrap();

    // Build and serialize the export
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
                dribble_events: events
                    .clone()
                    .iter()
                    .map(|e| {
                        let mut e = Into::<DribbleLabel>::into(e);
                        e.start_frame = e.start_frame.saturating_sub(20);
                        e.end_frame = e.end_frame.map(|f| f + 20);
                        e
                    })
                    .collect(),
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
    vid_name: String,
    video_data: VideoData,
    config: Config,
    video_mode: &String,
    mut dribble_detector: DribbleDetector,
    dribble_events_map: &Option<HashMap<String, Vec<(u32, u32)>>>,
) -> Option<(String, Vec<DribbleEvent>)> {
    // println!("All events {:?}", dribble_events_map);
    let review_mode = config.general.review_mode.unwrap_or(false);
    let log_level = config.general.log_level.clone();

    if config.general.log_level == "debug" {
        println!("Processing video {}", vid_name);
    }

    let mut vid_events = if review_mode {
        // if dribble_events_map.is_none()  {
        //     println!("Skipping video {}, found no dribble events file", vid_num);
        //     return None;
        // }

        let dribble_events = dribble_events_map.as_ref().unwrap();

        if let Some(event) = dribble_events.get(&vid_name) {
            println!(
                " * Found dribble events for video {}: {:?}",
                vid_name, event
            );
            event.clone()
        } else {
            // println!("Skipping video {}, found no dribble events", vid_num);
            return None;
        }
    } else {
        Vec::new()
    };

    let total_num_events = vid_events.len();
    let mut processed_events = 0;

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

    // Store a clone of vid_events
    let mut current_interval = if !vid_events.is_empty() {
        vid_events.remove(0)
    } else {
        (0, video_data.image_paths.len() as u32 - 1)
    };

    let mut start = current_interval.0;
    let mut end = current_interval.1;
    let mut first = true;

    // println!(" --- Start interval: {} - {}", start, end);

    let mut frame_num = start as usize;

    let iterator_start = video_data.image_paths.into_iter();

    let mut iterator = iterator_start.clone();
    let mut cur_path = iterator.next();

    let mut replay = false;

    let mut last_num = 0;

    while cur_path.is_some() {
        let image_path = cur_path.clone().unwrap();

        if EXIT_FLAG.load(Ordering::Relaxed) {
            break;
        }

        if review_mode {
            if processed_events >= total_num_events {
                println!("No more events to process (1)");
                break;
            }

            if frame_num < start as usize {
                // println!("continue");
                frame_num += 1;
                cur_path = iterator.next();
                continue;
            }
            if frame_num > end as usize {
                if processed_events >= total_num_events {
                    println!("No more events to process (2)");
                    break;
                }
                current_interval = vid_events.remove(0);
                start = current_interval.0;
                end = current_interval.1;

                first = true;
                frame_num = start as usize;

                continue;
            }
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
            frame_num += 1;
            cur_path = iterator.next();
            continue;
        }

        let dribble_frame = DribbleFrame {
            frame_number: frame_num as u32,
            players: player_models.unwrap(),
            ball: ball_model.unwrap_or(Ball { x: 0.0, y: 0.0 }),
        };

        let maybe_event = dribble_detector.process_frame(dribble_frame);

        if let Some(dribble_event) = maybe_event.clone() {
            if !replay && (dribble_event.detected_dribble || dribble_event.detected_tackle) {
                println!("\n\n\nDetected dribble event: {:?}", dribble_event.frames);
                detected_events.push(dribble_event);
            }
        }

        if config.general.video_mode == "display" {
            visualization_builder
                .add_frame(
                    &mut frame,
                    Some(image_id),
                    Some(&filtered_annotations),
                    &category_map,
                    maybe_event,
                )
                .expect("Failed to add frame");
        }

        let input_value =
            wait_for_keyboard_input(&config).expect("There was an error with keyboard input");

        match input_value {
            KeyboardInput::Quit => {
                EXIT_FLAG.store(true, Ordering::Relaxed);
                visualization_builder
                    .finish()
                    .expect("Failed to finish visualization");
                println!("Quitting...");
                break;
            }
            KeyboardInput::NextFrame => {
                cur_path = iterator.next();
            }
            KeyboardInput::PreviousFrame => {}
            KeyboardInput::NextClip => {
                processed_events += 1;

                visualization_builder
                    .finish()
                    .expect("Failed to finish visualization");

                replay = false;
                if review_mode && !vid_events.is_empty() {
                    current_interval = vid_events.remove(0);
                    start = current_interval.0;
                    end = current_interval.1;
                    first = false;
                }
            }
            _ => {}
        }

        frame_num += 1;
        if review_mode && frame_num >= end as usize {
            if !replay {
                println!("Displaying frames ({start}-{end})");
            }

            // visualization_builder
            //     .finish()
            //     .expect("Failed to finish visualization");

            frame_num = current_interval.0 as usize;

            iterator = iterator_start
                .clone()
                // .skip(frame_num)
                .collect::<Vec<_>>()
                .into_iter();
            cur_path = iterator.next();
            replay = true;
        }
    }

    let merged_events = combine_consecutive_events(detected_events);

    if log_level == "debug" {
        if review_mode {
            println!(" * Finished processing {} events\n", total_num_events);
        } else {
            println!(" * Finished processing {} events\n", merged_events.len());
        }
    }

    Some((file_name, merged_events))
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

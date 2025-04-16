use chrono::Utc;
use clap::Parser;
use dribbling_detection_algorithm::data::dataset::load_dribble_events_map;
use dribbling_detection_algorithm::data::download_data::download_and_extract_dataset;
use dribbling_detection_algorithm::data::models::{
    Annotation, DribbleEventsExport, DribbleLabel, ExportInfo, Image, ReviewedVideoData, VideoData,
    VideoDribbleEvents,
};
use dribbling_detection_algorithm::dribbling_detection::create_dribble_models::{
    get_ball_model, get_player_models,
};
use dribbling_detection_algorithm::dribbling_detection::dribble_detector::DribbleDetector;
use dribbling_detection_algorithm::dribbling_detection::dribble_models::{
    Ball, DribbleEvent, DribbleFrame,
};
use dribbling_detection_algorithm::utils::annotation_calculations::{
    compute_average_player_bbox_height, filter_annotations,
};
use dribbling_detection_algorithm::utils::keyboard_args::Args;
use dribbling_detection_algorithm::utils::keyboard_input::{
    wait_for_keyboard_input, KeyboardInput,
};
use dribbling_detection_algorithm::utils::video_processing::export_reviewed_data;
use dribbling_detection_algorithm::utils::visualizations::VisualizationBuilder;
use dribbling_detection_algorithm::{config::Config, data::dataset::Dataset};
use opencv::imgcodecs;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

static EXIT_FLAG: AtomicBool = AtomicBool::new(false);

fn main() {
    let start_time = Utc::now();
    let args = Args::parse();

    if args.download {
        println!("Data download initiated...");
        let config_content =
            fs::read_to_string("config.toml").expect("Unable to read the config file");
        let mut config: Config =
            toml::from_str(&config_content).expect("Unable to parse the config file");

        config = config.apply_env_overrides();

        if let Some(ip) = &args.input {
            config.data.data_path = ip.clone();
        }
        if let Some(op) = &args.output {
            config.data.output_path = op.clone();
        }

        let rt = Runtime::new().unwrap();
        rt.block_on(download_and_extract_dataset(&config));
        println!("Data download complete.");
    }

    println!("\nRunning dribbling detection");

    let config_content = fs::read_to_string("config.toml").expect("Unable to read the config file");
    let mut config: Config =
        toml::from_str(&config_content).expect("Unable to parse the config file");
    config = config.apply_env_overrides();

    // If the input or output paths were set on the command line, override the config
    if let Some(ip) = &args.input {
        println!("Overriding input path: {}", ip);
        config.data.data_path = ip.clone();
    }
    if let Some(op) = &args.output {
        println!("Overriding output path: {}", op);
        config.data.output_path = op.clone();
    }
    if args.review.is_some() && args.review.unwrap() {
        println!("Enabling review mode from keyboard args");
        config.general.review_mode = Some(true);
        config.general.video_mode = "display".to_string();
    }

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

    // let inner_rad = config.dribbling_detection.inner_radius;
    // let outer_rad = config.dribbling_detection.outer_radius;

    println!("Number of videos to process: {}", data_iter.len());

    // Shared map of all detected events
    let all_detected_events = Arc::new(Mutex::new(HashMap::new()));

    let dribble_events_map = if config.general.review_mode.unwrap_or(false) {
        load_dribble_events_map(&config)
    } else {
        None
    };

    let all_reviewed_video_data = if config.general.review_mode.unwrap_or(false) {
        Arc::new(Mutex::new(Some(Vec::new())))
    } else {
        Arc::new(Mutex::new(None))
    };

    // ---------------------------------------------------------------------------------------------
    // Define the per-video processing function
    // ---------------------------------------------------------------------------------------------
    let process_item = |video_data: &Result<VideoData, _>| {
        // Skip this video if it can't be unwrapped
        let video_data = match video_data {
            Ok(vd) => vd.clone(),
            Err(_) => return,
        };

        let category_map: HashMap<String, u32> = video_data
            .labels
            .categories
            .iter()
            .map(|c| (c.name.clone(), c.id))
            .collect();

        let average_bbox_height =
            compute_average_player_bbox_height(&video_data.labels.annotations, &category_map);
        let scale_factor = average_bbox_height * 0.2;

        let (inner_rad, outer_rad) = match config.dribbling_detection.use_2d {
            true => (
                config.dribbling_detection.inner_radius,
                config.dribbling_detection.outer_radius,
            ),
            false => (
                config.dribbling_detection.inner_radius * scale_factor,
                config.dribbling_detection.outer_radius * scale_factor,
            ),
        };

        let video_name = video_data
            .dir_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        // Build a DribbleDetector for this video
        // let video_name = format!("{:06}.jpg", vid_num);
        let dribble_detector = DribbleDetector::new(
            video_name.clone(),
            inner_rad,
            outer_rad,
            config.dribbling_detection.inner_threshold,
            config.dribbling_detection.outer_threshold,
            config.dribbling_detection.outer_in_threshold,
            config.dribbling_detection.outer_out_threshold,
            config.clone(),
        );

        // Check for early exit
        if EXIT_FLAG.load(Ordering::Relaxed) {
            return;
        }

        // Process the video
        let processed_video = detect_events(
            video_name,
            video_data,
            config.clone(),
            video_mode,
            dribble_detector.clone(),
            &dribble_events_map,
            all_reviewed_video_data.clone(),
            inner_rad,
            outer_rad,
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
        data_iter.par_iter().for_each(process_item);
    } else {
        data_iter.iter().for_each(process_item);
    }

    if config.general.review_mode.unwrap_or(false) {
        let cur_time = Utc::now();
        let duration = cur_time - start_time;
        let all_reviewed_video_data = Arc::try_unwrap(all_reviewed_video_data)
            .unwrap()
            .into_inner()
            .unwrap()
            .unwrap_or_default();

        println!(
            "\n\nReview mode done in {}H:{}M:{}S",
            duration.num_hours(),
            duration.num_minutes() % 60,
            duration.num_seconds() % 60
        );
        
        let total_dribbles: usize = all_reviewed_video_data.iter().map(|r| r.dribble_data.len()).sum();
        let total_tackles: usize = all_reviewed_video_data.iter().map(|r| r.tackle_data.len()).sum();
        let total_others: usize = all_reviewed_video_data.iter().map(|r| r.other_data.len()).sum();

        println!(
            "Approved {} dribbles, {} tackles and disaproved {} events",
            total_dribbles,
            total_tackles,
            total_others,
        );

        println!(
            "\n\nExporting reviewed data to {}...",
            config.data.output_path
        );
    
        if let Err(e) = export_reviewed_data(
            Path::new(&config.data.output_path),
            &all_reviewed_video_data,
        ) {
            eprintln!("Error exporting reviewed data: {}", e);
        }
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
            .map(|(video_id, events)| VideoDribbleEvents {
                video_id: video_id.clone(),
                dribble_events: events
                    .clone()
                    .iter()
                    .map(|e| Into::<DribbleLabel>::into(e))
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
fn detect_events(
    vid_name: String,
    video_data: VideoData,
    config: Config,
    video_mode: &String,
    mut dribble_detector: DribbleDetector,
    dribble_events_map: &Option<HashMap<String, Vec<(u32, u32)>>>,
    all_reviewed_video_data: Arc<Mutex<Option<Vec<ReviewedVideoData>>>>,
    inner_rad: f64,
    outer_rad: f64,
) -> Option<(String, Vec<DribbleEvent>)> {
    let review_mode = config.general.review_mode.unwrap_or(false);
    let log_level = config.general.log_level.clone();

    let mut reviewed_video_data = review_mode.then(|| ReviewedVideoData::default());

    if config.general.log_level == "debug" {
        println!("Processing video {}", vid_name);
    }

    let mut vid_events = if review_mode {
        if dribble_events_map.is_none() {
            println!("Skipping video {}, found no dribble events file", vid_name);
            return None;
        }

        let dribble_events = dribble_events_map.as_ref().unwrap();

        if let Some(event) = dribble_events.get(&vid_name) {
            println!(
                " * Found dribble events for video {}: {:?}",
                vid_name, event
            );
            event.clone()
        } else {
            return None;
        }
    } else {
        Vec::new()
    };

    let total_num_events = vid_events.len();
    let processed_events = 0;

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
    // let file_name = format!("video_{}", vid_num);
    let file_name = vid_name.clone();

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

    let mut frame_num;

    let iterator_start = video_data.image_paths.clone().into_iter();

    let mut iterator = iterator_start.clone();
    let mut cur_path = iterator.next();

    let mut replay = false;

    let mut current_frames = current_interval.clone();

    while cur_path.is_some() && end != 0 {

        if current_frames != current_interval {
            println!("Displaying frames ({start}-{end})");
            current_frames = current_interval.clone();
        };
        
        let image_path = cur_path.clone().unwrap();
        let image_name = cur_path
            .clone()?
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        frame_num = image_name
            .parse::<usize>()
            .expect("Failed to parse frame number from image name");

        if EXIT_FLAG.load(Ordering::Relaxed) {
            break;
        }

        if review_mode {
            if processed_events >= total_num_events {
                println!("No more events to process (1)");
                break;
            }

            if frame_num < start as usize {
                cur_path = iterator.next();
                continue;
            }
            if frame_num > end as usize {
                if processed_events >= total_num_events {
                    println!("No more events to process (2)");
                    break;
                }
                current_interval = if !vid_events.is_empty() {
                    vid_events.remove(0)
                } else {
                    (0, 0)
                };
                start = current_interval.0;
                end = current_interval.1;
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
        let ball_model = get_ball_model(&category_map, &filtered_annotations, &config);
        let player_models = get_player_models(&category_map, &filtered_annotations, &config);

        if player_models.is_none() {
            println!("(In main): No players found in frame. Skipping frame...");
            // frame_num += 1;
            cur_path = iterator.next();
            continue;
        }

        let dribble_frame = DribbleFrame {
            frame_number: frame_num as u32,
            players: player_models.unwrap(),
            ball: ball_model.unwrap_or(Ball { x: 0.0, y: 0.0 }),
        };

        let potential_event = dribble_detector.process_frame(dribble_frame);

        if let Some(mut dribble_event) = potential_event.clone() {
            if !replay && (dribble_event.detected_dribble || dribble_event.detected_tackle) {
                // println!("\n\n\nDetected dribble event: {:?}", dribble_event.frames);
                let extra_frames_before = 10;
                let extra_frames_after = 20;
                dribble_event.start_frame = dribble_event.start_frame.saturating_sub(extra_frames_before);

                if let Some(cur_end) = dribble_event.end_frame {
                    dribble_event.end_frame = Some(cur_end + extra_frames_after);
                };

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
                    inner_rad,
                    outer_rad,
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
                cur_path = iterator.next();
                replay = false;

                current_interval = if !vid_events.is_empty() {
                    vid_events.remove(0)
                } else {
                    (0, 0)
                };

                start = current_interval.0;
                end = current_interval.1;

                visualization_builder
                    .finish()
                    .expect("Failed to finish visualization");
            }
            KeyboardInput::Dribble => {
                if review_mode {
                    println!("Adding dribble event");
                    cur_path = iterator.next();
                    replay = false;

                    current_interval = if !vid_events.is_empty() {
                        vid_events.remove(0)
                    } else {
                        (0, 0)
                    };

                    start = current_interval.0;
                    end = current_interval.1;

                    let filtered_video_data = filter_video_data(video_data.clone(), start, end);
                    reviewed_video_data
                        .as_mut()
                        .unwrap()
                        .dribble_data
                        .push(filtered_video_data);

                    all_reviewed_video_data
                        .lock()
                        .unwrap()
                        .as_mut()
                        .unwrap()
                        .push(reviewed_video_data.clone().unwrap());
                    continue;
                }
            }
            KeyboardInput::Tackle => {
                if review_mode {
                    println!("Adding tackle event");
                    let filtered_video_data = filter_video_data(video_data.clone(), start, end);
                    reviewed_video_data
                        .as_mut()
                        .unwrap()
                        .tackle_data
                        .push(filtered_video_data);
                    cur_path = iterator.next();
                    replay = false;

                    current_interval = if !vid_events.is_empty() {
                        vid_events.remove(0)
                    } else {
                        (0, 0)
                    };

                    start = current_interval.0;
                    end = current_interval.1;

                    all_reviewed_video_data
                        .lock()
                        .unwrap()
                        .as_mut()
                        .unwrap()
                        .push(reviewed_video_data.clone().unwrap());
                    continue;
                }
            }
            KeyboardInput::None => {
                if review_mode {
                    println!("Adding other event");
                    let filtered_video_data = filter_video_data(video_data.clone(), start, end);

                    reviewed_video_data
                        .as_mut()
                        .unwrap()
                        .other_data
                        .push(filtered_video_data);
                    cur_path = iterator.next();
                    replay = false;

                    current_interval = if !vid_events.is_empty() {
                        vid_events.remove(0)
                    } else {
                        (0, 0)
                    };

                    start = current_interval.0;
                    end = current_interval.1;

                    all_reviewed_video_data
                        .lock()
                        .unwrap()
                        .as_mut()
                        .unwrap()
                        .push(reviewed_video_data.clone().unwrap());

                    continue;
                }
            }
        }

        // Replay clip
        if review_mode && (frame_num >= end as usize || cur_path.is_none()) {

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
/// is immediately after (or within max_event_gap) the end of the previous event,
/// and both events are of the same type (dribble or tackle).
fn combine_consecutive_events(mut events: Vec<DribbleEvent>) -> Vec<DribbleEvent> {
    events.sort_by_key(|e| e.start_frame);

    let max_event_gap = 8;

    let mut merged: Vec<DribbleEvent> = Vec::new();
    for event in events {
        if let Some(last) = merged.last_mut() {
            if let Some(last_end) = last.end_frame {
                let same_type = (last.detected_tackle && event.detected_tackle)
                    || (last.detected_dribble && event.detected_dribble);

                if event.start_frame <= last_end + max_event_gap && same_type {
                    last.extend(&event);

                    if let Some(end) = event.end_frame {
                        last.end_frame = Some(end);
                    }

                    last.detected_tackle |= event.detected_tackle;
                    last.detected_dribble |= event.detected_dribble;
                    last.ever_contested |= event.ever_contested;
                    continue;
                }
            }
        }
        merged.push(event);
    }
    merged
}


fn filter_video_data(video_data: VideoData, start: u32, end: u32) -> VideoData {
    let mut filtered_data = VideoData::default();
    filtered_data.dir_path = video_data.dir_path.clone();

    // Helper to parse the zero-padded frame number from the filename (e.g. "0001.jpg" -> 1).
    let in_range = |name: &str| -> bool {
        if let Some(stem) = std::path::Path::new(name).file_stem() {
            if let Ok(num) = stem.to_string_lossy().parse::<u32>() {
                return num >= start && num <= end;
            }
        }
        false
    };

    let new_image_paths: Vec<PathBuf> = video_data
        .image_paths
        .into_iter()
        .filter(|p| {
            if let Some(fname) = p.file_name().map(|f| f.to_string_lossy().to_string()) {
                in_range(&fname)
            } else {
                false
            }
        })
        .collect();

    let new_images: Vec<Image> = video_data
        .labels
        .images
        .into_iter()
        .filter(|img| in_range(&img.file_name))
        .collect();

    let valid_ids: std::collections::HashSet<String> =
        new_images.iter().map(|img| img.image_id.clone()).collect();

    let new_annotations: Vec<Annotation> = video_data
        .labels
        .annotations
        .into_iter()
        .filter(|ann| valid_ids.contains(&ann.image_id))
        .collect();

    filtered_data.image_paths = new_image_paths;
    filtered_data.labels.images = new_images;
    filtered_data.labels.annotations = new_annotations;
    filtered_data.labels.categories = video_data.labels.categories;

    filtered_data
}

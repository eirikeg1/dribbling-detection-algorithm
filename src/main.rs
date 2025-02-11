use dribbling_detection_algorithm::data::download_data::download_and_extract_dataset;
use dribbling_detection_algorithm::data::models::Annotation;
use dribbling_detection_algorithm::dribbling_detection::create_dribble_models::{
    get_ball_model, get_player_models,
};
use dribbling_detection_algorithm::dribbling_detection::dribble_detector::DribbleDetector;
use dribbling_detection_algorithm::dribbling_detection::dribble_models::{Ball, DribbleFrame};
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

    let mut dribble_detector = DribbleDetector::new(
        inner_rad,
        outer_rad,
        config.dribbling_detection.inner_threshold,
        config.dribbling_detection.outer_threshold,
    );

    // Iterate over videos
    for (vid_num, video_data) in data_iter.enumerate() {
        let video_data = video_data.unwrap();
        // let image_dir = video_data.labels.info.im_dir.clone();
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

            if let Err(err) = std::fs::metadata(&image_path) {
                eprintln!(
                    "File '{:?}' does not exist or cannot be accessed: {}",
                    image_path, err
                );
            } else {
                // proceed to read
                let frame =
                    imgcodecs::imread(image_path.to_str().unwrap(), imgcodecs::IMREAD_COLOR)
                        .expect(format!("Failed to read image '{:?}'", image_path).as_str());

                if frame.empty() {
                    eprintln!(
                        "OpenCV could not decode the image, returning an empty matrix. Path: {:?}",
                        image_path.to_str().unwrap()
                    );
                }
            }

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

            // if ball_model.is_none() {
            //     println!("(In main): No ball  found in frame. Skipping frame...");
            //     continue;
            // }

            let dribble_frame = DribbleFrame {
                frame_number: frame_num as u32,
                players: player_models.unwrap(),
                ball: ball_model.unwrap_or(Ball { x: 0.0, y: 0.0 }),
            };
            let dribble_event = dribble_detector.process_frame(dribble_frame);

            if dribble_event.is_some() && dribble_event.as_ref().unwrap().detected_dribble {
                println!(
                    " * Dribble event detected: {:?}",
                    dribble_event.as_ref().unwrap().frames
                );
            } else if dribble_event.is_some() && dribble_event.as_ref().unwrap().detected_tackle {
                println!(
                    " * Tackle event detected: {:?}",
                    dribble_event.as_ref().unwrap().frames
                );
            }
            // else if dribble_event.is_some() && dribble_event.as_ref().unwrap().ever_contested {
            //     println!(" * Contested dribble event detected.");
            // } else if dribble_event.is_some() {
            //     println!(" * Uncontested dribble event detected.");
            // } else {
            //     println!(" * No dribble event detected.");
            // }

            visualization_builder
                .add_frame(
                    &mut frame,
                    Some(image_id),
                    Some(&filtered_annotations),
                    &category_map,
                    dribble_event,
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

        visualization_builder
            .finish()
            .expect("Failed to finish visualization");
    }
}

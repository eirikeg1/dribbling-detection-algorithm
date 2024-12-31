use dribbling_detection_algorithm::domain::data::download_data::download_and_extract_dataset;
use dribbling_detection_algorithm::domain::data::models::Annotation;
use dribbling_detection_algorithm::domain::events::create_drible_models::{
    get_ball_model, get_player_models,
};
use dribbling_detection_algorithm::domain::events::drible_detector::DribbleDetector;
use dribbling_detection_algorithm::domain::events::drible_models::DribleFrame;
use dribbling_detection_algorithm::utils::visualizations::VisualizationBuilder;
use dribbling_detection_algorithm::{config::Config, domain::data::dataset::Dataset};
use opencv::{highgui, imgcodecs};
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
    let train_iter = dataset.iter_subset(&"train");
    let inner_rad = config.dribbling_detection.inner_radius;
    let outer_rad = config.dribbling_detection.outer_radius;

    let mut dribble_detector = DribbleDetector::new(inner_rad, outer_rad);

    // Iterate over videos
    for (i, video_data) in train_iter.enumerate().skip(1) {
        if i % 1 == 0 {
            println!("Processing  video {i}...");
        }
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
        let file_name = format!("video_{}", i);

        let mut visualization_builder =
            VisualizationBuilder::new(video_mode.as_str(), &file_name, &config)
                .expect("Failed to create visualization builder");

        for image_path in video_data.image_paths.into_iter() {
            let image_file_name = image_path
                .to_string_lossy()
                .split('/')
                .last()
                .unwrap_or("")
                .to_string();

            let image_id = image_map.get(&image_file_name).unwrap_or(&image_file_name);

            let mut frame =
                imgcodecs::imread(image_path.to_str().unwrap(), imgcodecs::IMREAD_COLOR).unwrap();

            let filtered_annotations = annotations
                .iter()
                .filter(|a| a.image_id == *image_id)
                .cloned()
                .collect::<Vec<Annotation>>();

            let ball_model = get_ball_model(&category_map, &filtered_annotations);
            let player_models = get_player_models(&category_map, &filtered_annotations);

            if ball_model.is_none() || player_models.is_none() {
                println!("(In main): No ball or player found in frame. Skipping...");
                continue;
            }

            let drible_frame = DribleFrame {
                frame_number: i as u32,
                players: player_models.unwrap(),
                ball: ball_model.unwrap(),
            };
            let drible_event = dribble_detector.process_frame(drible_frame);

            visualization_builder
                .add_frame(
                    &mut frame,
                    Some(image_id),
                    Some(&filtered_annotations),
                    &category_map,
                    drible_event,
                )
                .expect("Failed to add frame");

            let _ = highgui::wait_key(0);

            if let Ok(113) = highgui::wait_key(30) {
                return; // If 'q' (ASCII 113) is pressed, exit the program
            } else {
                continue; // Else continue
            }
        }

        visualization_builder
            .finish()
            .expect("Failed to finish visualization");
    }
}

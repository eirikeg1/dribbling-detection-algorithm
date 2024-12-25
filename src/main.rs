use dribbling_detection_algorithm::domain::data::download_data::download_and_extract_dataset;
use dribbling_detection_algorithm::domain::data::models::Annotation;
use dribbling_detection_algorithm::domain::events::drible_detector::DribbleDetector;
use dribbling_detection_algorithm::utils::visualizations::VisualizationBuilder;
use dribbling_detection_algorithm::{config::Config, domain::data::dataset::Dataset};
use opencv::core::Mat;
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
    let train_iter = dataset.iter_subset(&"train");
    let inner_rad = config.dribbling_detection.inner_radius;
    let outer_rad = config.dribbling_detection.outer_radius;

    let mut dribble_detector = DribbleDetector::new(inner_rad, outer_rad);

    // Iterate over videos
    for (i, video_data) in train_iter.enumerate() {
        if i % 1 == 0 {
            println!("Processing  video {i}...");
        }
        let video_data = video_data.unwrap();
        let image_dir = video_data.labels.info.im_dir.clone();
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
            let mut frame = Mat::default();

            frame =
                imgcodecs::imread(image_path.to_str().unwrap(), imgcodecs::IMREAD_COLOR).unwrap();

            let filtered_annotations = annotations
                .iter()
                .filter(|a| a.image_id == *image_id)
                .cloned()
                .collect::<Vec<Annotation>>();

            visualization_builder
                .add_frame(&mut frame, Some(image_id), Some(&filtered_annotations))
                .expect("Failed to add frame");
        }

        visualization_builder
            .finish()
            .expect("Failed to finish visualization");
    }
}

// fn old_code() {
//     let ball_id: u32 = match category_map.get("ball") {
//         Some(id) => *id,
//         None => continue,
//     };
//     let ball: Vec<Ball> = annotations
//         .iter()
//         .filter_map(|a| {
//             if a.category_id == ball_id {
//                 let (x, y) = calculate_bbox_pitch_center(a.clone())?;
//                 Some(Ball {
//                     x: x,
//                     y: y,
//                 })
//             } else {
//                 None
//             }
//         })
//         .collect();

//     let player_id: u32 = match category_map.get("player") {
//         Some(id) => *id,
//         None => continue,
//     };
//     let players: Vec<Player> = annotations
//         .iter()
//         .filter_map(|a| {
//             if a.category_id != player_id {
//                 let (x, y) = calculate_bbox_pitch_center(a.clone())?;
//                 Some(Player {
//                     id: a.track_id?,
//                     x: x,
//                     y: y,
//                     velocity: (0.0, 0.0),
//                 })
//             } else {
//                 None
//             }
//         })
//         .collect();

//     let frame = Frame {
//         frame_number: i as u32,
//         players: players,
//         ball: ball[0].clone(),
//     };

//     println!("Processing frame: {:?}", frame);
//     dribble_detector.process_frame(frame);
// }

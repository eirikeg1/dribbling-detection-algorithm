use dribbling_detection_algorithm::domain::data::download_data::download_and_extract_dataset;
use dribbling_detection_algorithm::domain::data::models::Annotation;
use dribbling_detection_algorithm::domain::events::drible_detector::DribbleDetector;
use dribbling_detection_algorithm::{
    config::Config, domain::data::dataset::Dataset, utils::visualizations::visualize_or_store_video,
};
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
    let output_path: &String = &config.data.output_path;

    let dataset = Dataset::new(config.clone());
    let train_iter = dataset.iter_subset(&"train");
    let inner_rad = config.dribbling_detection.inner_radius;
    let outer_rad = config.dribbling_detection.outer_radius;

    let dribble_detector = DribbleDetector::new(inner_rad, outer_rad);

    for (i, video_data) in train_iter.enumerate() {
        if i % 1 == 0 {
            println!("Processing {}", i);
        }
        let video_data = video_data.unwrap();
        let image_dir = video_data.labels.info.im_dir.clone();
        println!("Processing video: {}", video_data.dir_path.display());

        // let annotations: Vec<Annotation> = video_data.labels.annotations.clone();
        // let players = todo!("Get the player annotations from the config file");
        // let ball_id: u32 = todo!("Get the ball id from the config file");
        // let ball_annotations: Vec<Annotation> = annotations
        //     .iter()
        //     .filter(|a| a.category_id == ball_id)
        //     .cloned()
        //     .collect();
        
        // let frame = todo!("Load the frame from the video data");
        
        // dribble_detector.process_frame(frame);

        visualize_or_store_video(
            std::path::Path::new(&format!("{}/{}", video_data.dir_path.display(), image_dir.unwrap_or("img1".to_string()))),
            video_data.labels.annotations.as_slice(),
            video_data.labels.images.as_slice(),
            video_mode.as_str(),
            output_path.as_str(),
            &format!("video_{}", i),
            &config,
        )
        .unwrap();
    }
}

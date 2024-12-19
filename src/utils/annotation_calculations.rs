use crate::domain::data::models::Annotation;

/// Finds the closest annotation to the base annotation
pub fn annotation_comparator(
    base_annotation: Annotation,
    other_annotations: Vec<Annotation>,
) -> Option<Annotation> {
    let mut closest_annotation = None;
    let mut closest_distance = f64::MAX;

    for annotation in other_annotations {
        let distance = calculate_annotation_distance(base_annotation.clone(), annotation.clone())?;
        if distance < closest_distance {
            closest_distance = distance;
            closest_annotation = Some(annotation);
        }
    }

    closest_annotation
}

/// Determines if annotation center is within range
pub fn is_within_range(
    base_annotation: Annotation,
    other_annotation: Annotation,
    range: f64,
) -> Option<bool> {
    let distance = calculate_annotation_distance(base_annotation, other_annotation)?;
    Some(distance < range)
}

/// Euclidean distance between two annotations
pub fn calculate_annotation_distance(
    annotation_1: Annotation,
    annotation_2: Annotation,
) -> Option<f64> {
    let coords_1 = calculate_bbox_pitch_center(annotation_1)?;
    let coords_2 = calculate_bbox_pitch_center(annotation_2)?;

    Some(((coords_2.0 - coords_1.0).powi(2) + (coords_2.1 - coords_1.1).powi(2)).sqrt())
}

/// Calculate the center of the BboxPitch
pub fn calculate_bbox_pitch_center(annotation: Annotation) -> Option<(f64, f64)> {
    let bbox = annotation.bbox_pitch?;

    // Calculate the geometric center
    let x_center = (bbox.x_bottom_left + bbox.x_bottom_right) / 2.0;
    let y_center = (bbox.y_bottom_left + bbox.y_bottom_right) / 2.0;

    Some((x_center, y_center))
}

#[cfg(test)]
mod tests {
    use crate::domain::data::models::BboxPitch;

    use super::*;

    fn create_annotation(x_bl: f64, y_bl: f64, x_br: f64, y_br: f64) -> Annotation {
        Annotation {
            bbox_pitch: Some(BboxPitch {
                x_bottom_left: x_bl,
                y_bottom_left: y_bl,
                x_bottom_right: x_br,
                y_bottom_right: y_br,
                x_bottom_middle: (x_bl + x_br) / 2.0,
                y_bottom_middle: (y_bl + y_br) / 2.0,
            }),
            ..Default::default()
        }
    }

    #[test]
    fn test_calculate_bbox_pitch_center() {
        let annotation = create_annotation(0.0, 0.0, 2.0, 2.0);
        let center = calculate_bbox_pitch_center(annotation).unwrap();
        assert_eq!(center, (1.0, 1.0));
    }

    #[test]
    fn test_calculate_annotation_distance() {
        let annotation_1 = create_annotation(0.0, 0.0, 2.0, 2.0); // center = (1, 1)
        let annotation_2 = create_annotation(3.0, 3.0, 5.0, 5.0); // center = (4, 4)
        let distance = calculate_annotation_distance(annotation_1, annotation_2).unwrap();
        assert!((distance - 4.242).abs() < 0.001); // sqrt((4-1)^2 + (4-1)^2) = 4.242
    }

    #[test]
    fn test_is_within_range() {
        let annotation_1 = create_annotation(0.0, 0.0, 2.0, 2.0); // center = (1, 1)
        let annotation_2 = create_annotation(1.0, 1.0, 3.0, 3.0); // center = (2, 2)
        let result = is_within_range(annotation_1, annotation_2, 2.0).unwrap();
        assert!(result); // sqrt((2-1)^2 + (2-1)^2) = sqrt(2) = 1.414 < 2.0
    }

    #[test]
    fn test_annotation_comparator() {
        let base_annotation = create_annotation(0.0, 0.0, 2.0, 2.0); // center = (1, 1)
        let other_annotations = vec![
            create_annotation(3.0, 3.0, 5.0, 5.0), // center = (4, 4)
            create_annotation(1.0, 1.0, 3.0, 3.0), // center = (2, 2)
            create_annotation(6.0, 6.0, 8.0, 8.0), // center = (7, 7)
        ];
        let closest_annotation = annotation_comparator(base_annotation, other_annotations).unwrap();
        let expected_annotation = create_annotation(1.0, 1.0, 3.0, 3.0);
        assert_eq!(
            closest_annotation.bbox_pitch.unwrap().x_bottom_left,
            expected_annotation.bbox_pitch.unwrap().x_bottom_left
        );
    }
}
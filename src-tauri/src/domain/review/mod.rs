// Review domain module
// Configuration and logic for AI and human code review

pub mod config;
pub mod review_points;

pub use config::ReviewSettings;
pub use review_points::{
    get_review_point_type, is_complex_task, is_destructive_task, should_auto_insert_review_point,
    ReviewPointConfig, ReviewPointType,
};

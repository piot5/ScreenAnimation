//! Tests for the SequenceManager module.

use screen_animation::animation::sequence::SequenceManager;
use screen_animation::loader::{Config, FlowPackage, Step};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_sequence_package() -> FlowPackage {
        let mut steps = Vec::new();
        steps.push(Step {
            shader_entry: "test_entry_1".to_string(),
            duration_ms: 1000,
        });
        steps.push(Step {
            shader_entry: "test_entry_2".to_string(),
            duration_ms: 2000,
        });

        FlowPackage {
            config: Config {
                sequence: steps,
                ..Default::default()
            },
            sounds: HashMap::new(),
            image_data: None,
            textures: HashMap::new(),
            shader_src: String::new(),
        }
    }

    #[test]
    fn test_sequence_manager_creation() {
        let flow = create_test_sequence_package();
        let manager = SequenceManager::new(&flow);
        assert_eq!(manager.steps.len(), 2);
    }

    #[test]
    fn test_sequence_manager_select_step() {
        let flow = create_test_sequence_package();
        let mut manager = SequenceManager::new(&flow);

        // Test selection of the first step
        let step = manager.select_step(500.0);
        assert!(step.is_some());
        assert_eq!(step.unwrap().0, 0);

        // Test selection of the second step
        let step = manager.select_step(2500.0);
        assert!(step.is_some());
        assert_eq!(step.unwrap().0, 1);

        // Test selection before the sequence starts
        let step = manager.select_step(-100.0);
        assert!(step.is_none());

        // Test selection after the sequence ends
        let step = manager.select_step(5000.0);
        assert!(step.is_none());
    }

    #[test]
    fn test_sequence_manager_total_duration() {
        let flow = create_test_sequence_package();
        let manager = SequenceManager::new(&flow);
        assert_eq!(manager.total_duration_ms(), 3000);
    }

    #[test]
    fn test_sequence_manager_is_finished() {
        let flow = create_test_sequence_package();
        let manager = SequenceManager::new(&flow);

        // Sequence should not be finished before total duration
        assert!(!manager.is_finished(2000.0));

        // Sequence should be finished after total duration
        assert!(manager.is_finished(4000.0));
    }

    #[test]
    fn test_sequence_manager_collect_entries() {
        let flow = create_test_sequence_package();
        let manager = SequenceManager::new(&flow);
        let entries = manager.collect_entries();
        assert_eq!(entries.len(), 2);
        assert!(entries.contains(&"test_entry_1".to_string()));
        assert!(entries.contains(&"test_entry_2".to_string()));
    }
}
#[cfg(test)]
mod tests {
    
    use crate::events::{Event, SimulationEvent};
    

    #[test]
    fn test_event_creation() {
        let event = Event::new(SimulationEvent::SimulationStarted {
            step_count: 5,
            years_per_step: 1000.0,
        });

        match &event.event {
            SimulationEvent::SimulationStarted { step_count, years_per_step } => {
                assert_eq!(*step_count, 5);
                assert_eq!(*years_per_step, 1000.0);
            },
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_event_metadata() {
        let event = Event::new(SimulationEvent::StepStarted { step: 1, year: 1000.0 })
            .with_step(1)
            .with_component("TestComponent".to_string());

        assert_eq!(event.metadata.step, Some(1));
        assert_eq!(event.metadata.component, Some("TestComponent".to_string()));
    }

}

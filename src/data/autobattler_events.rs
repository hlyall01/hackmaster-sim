use crate::core::gameplay::EventCatalog;
use crate::data::resolve_data_path;
use std::fs;

const EMBEDDED_AUTOBATTLER_EVENTS_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/data/autobattler/events_v1.json"
));
const EMBEDDED_AUTOBATTLER_EVENTS_HANDCRAFTED_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/data/autobattler/events_v1_handcrafted.json"
));

pub fn load_autobattler_events(path: &str) -> Result<EventCatalog, String> {
    let data = fs::read_to_string(resolve_data_path(path))
        .unwrap_or_else(|_| EMBEDDED_AUTOBATTLER_EVENTS_JSON.to_string());
    let mut catalog: EventCatalog = serde_json::from_str(&data).map_err(|err| err.to_string())?;
    let handcrafted = fs::read_to_string(resolve_data_path(
        "data/autobattler/events_v1_handcrafted.json",
    ))
    .unwrap_or_else(|_| EMBEDDED_AUTOBATTLER_EVENTS_HANDCRAFTED_JSON.to_string());
    let handcrafted_catalog: EventCatalog =
        serde_json::from_str(&handcrafted).map_err(|err| err.to_string())?;
    catalog.events.extend(handcrafted_catalog.events);
    Ok(catalog)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_catalog_has_required_depth_and_size() {
        let catalog =
            load_autobattler_events("data/autobattler/events_v1.json").expect("load events");
        assert_eq!(catalog.version, 1);
        assert!(
            catalog.events.len() >= 400,
            "expected at least 400 events, got {}",
            catalog.events.len()
        );
    }

    #[test]
    fn embedded_catalog_includes_chain_and_fight_events() {
        let catalog =
            load_autobattler_events("data/autobattler/events_v1.json").expect("load events");
        let chain_count = catalog
            .events
            .iter()
            .filter(|event| !event.requires_flags.is_empty())
            .count();
        assert!(
            chain_count > 0,
            "expected at least one prerequisite-gated chain event"
        );
        let fight_count = catalog
            .events
            .iter()
            .filter(|event| {
                event
                    .choices
                    .iter()
                    .any(|choice| choice.success.trigger_fight || choice.failure.trigger_fight)
            })
            .count();
        assert!(
            fight_count > 0,
            "expected at least one event branch that escalates into a fight"
        );
    }

    #[test]
    fn handcrafted_embedded_catalog_has_200_plus_entries() {
        let catalog: EventCatalog =
            serde_json::from_str(EMBEDDED_AUTOBATTLER_EVENTS_HANDCRAFTED_JSON)
                .expect("parse handcrafted events");
        assert!(
            catalog.events.len() >= 200,
            "expected at least 200 handcrafted events, got {}",
            catalog.events.len()
        );
    }
}

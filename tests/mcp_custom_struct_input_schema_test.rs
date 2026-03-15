//! Tests that custom struct types used as tool input parameters generate
//! correct JSON Schema via schemars, instead of falling back to "string".
//!
//! This covers:
//! - `Vec<CustomStruct>` parameters (the primary use case)
//! - Direct `CustomStruct` parameters
//! - `Option<CustomStruct>` parameters
//! - Nested structs

use serde_json::Value;
use turul_mcp_builders::prelude::*;
use turul_mcp_derive::mcp_tool;
use turul_mcp_protocol::McpResult;

/// A custom struct used as a tool input parameter
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
struct ObserverPoint {
    /// Latitude in degrees
    latitude: f64,
    /// Longitude in degrees
    longitude: f64,
    /// Optional label
    label: Option<String>,
}

/// Tool that takes Vec<CustomStruct> as parameter
#[mcp_tool(name = "batch_observe", description = "Process batch of observer points")]
async fn batch_observe(points: Vec<ObserverPoint>) -> McpResult<String> {
    Ok(format!("Processed {} points", points.len()))
}

/// Tool that takes a single CustomStruct as parameter
#[mcp_tool(
    name = "single_observe",
    description = "Process single observer point"
)]
async fn single_observe(point: ObserverPoint) -> McpResult<String> {
    Ok(format!("Observed at {}, {}", point.latitude, point.longitude))
}

/// Tool that takes Option<CustomStruct>
#[mcp_tool(
    name = "optional_observe",
    description = "Process optional observer point"
)]
async fn optional_observe(
    name: String,
    point: Option<ObserverPoint>,
) -> McpResult<String> {
    Ok(format!(
        "Observer {} at {:?}",
        name,
        point.map(|p| (p.latitude, p.longitude))
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_custom_struct_schema_is_array_of_objects() {
        let tool = BatchObserveToolImpl;
        let schema = tool.input_schema();
        let schema_json: Value = serde_json::to_value(schema).unwrap();

        // The "points" property should be an array, not a string
        let points_schema = &schema_json["properties"]["points"];
        assert_eq!(
            points_schema["type"], "array",
            "Vec<ObserverPoint> must be type=array, got: {points_schema}"
        );

        // The items should be type=object with properties, not type=string
        let items = &points_schema["items"];
        assert_eq!(
            items["type"], "object",
            "Items of Vec<ObserverPoint> must be type=object, got: {items}"
        );

        // The object should have the struct's fields as properties
        let properties = &items["properties"];
        assert!(
            properties["latitude"].is_object(),
            "Must have 'latitude' property: {properties}"
        );
        assert!(
            properties["longitude"].is_object(),
            "Must have 'longitude' property: {properties}"
        );
        assert!(
            properties["label"].is_object(),
            "Must have 'label' property: {properties}"
        );
    }

    #[test]
    fn test_single_custom_struct_schema_is_object() {
        let tool = SingleObserveToolImpl;
        let schema = tool.input_schema();
        let schema_json: Value = serde_json::to_value(schema).unwrap();

        // The "point" property should be an object with properties
        let point_schema = &schema_json["properties"]["point"];
        assert_eq!(
            point_schema["type"], "object",
            "ObserverPoint must be type=object, got: {point_schema}"
        );
        assert!(
            point_schema["properties"]["latitude"].is_object(),
            "Must have 'latitude': {point_schema}"
        );
    }

    #[test]
    fn test_option_custom_struct_schema_is_object() {
        let tool = OptionalObserveToolImpl;
        let schema = tool.input_schema();
        let schema_json: Value = serde_json::to_value(schema).unwrap();

        // "name" should be a string
        assert_eq!(
            schema_json["properties"]["name"]["type"], "string",
            "name must be string"
        );

        // "point" should be an object (Option unwraps to the inner type's schema)
        let point_schema = &schema_json["properties"]["point"];
        assert_eq!(
            point_schema["type"], "object",
            "Option<ObserverPoint> must be type=object, got: {point_schema}"
        );
    }

    #[test]
    fn test_vec_custom_struct_deserializes_from_json_array() {
        // Verify that the schema is truthful — serde can actually deserialize
        // a JSON array of objects into Vec<ObserverPoint>
        let json_input = serde_json::json!([
            {"latitude": 40.7128, "longitude": -74.0060, "label": "NYC"},
            {"latitude": 51.5074, "longitude": -0.1278}
        ]);

        let points: Vec<ObserverPoint> = serde_json::from_value(json_input).unwrap();
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].label, Some("NYC".to_string()));
        assert_eq!(points[1].label, None);
    }
}

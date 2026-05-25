use crate::error::{AppError, AppResult};
use crate::protocol::messages::{CreateInstanceRequest, CreateInstanceResponse, CreateProperty};
use std::path::PathBuf;

pub fn run(
    port: u16,
    studio: Option<String>,
    class_name: Option<String>,
    parent_path: Option<String>,
    name: Option<String>,
    properties: Vec<String>,
    json_file: Option<PathBuf>,
    json_output: bool,
) -> AppResult<()> {
    let request = if let Some(json_file) = json_file {
        let mut request: CreateInstanceRequest =
            serde_json::from_str(&std::fs::read_to_string(&json_file)?)?;
        if studio.is_some() {
            request.studio = studio;
        }
        request
    } else {
        CreateInstanceRequest {
            studio,
            parent_path: parent_path.unwrap_or_else(|| "Workspace".to_string()),
            class_name: class_name.ok_or_else(|| {
                AppError::Other("--class is required unless --json is used".into())
            })?,
            name: name.unwrap_or_else(|| "CreatedInstance".to_string()),
            properties: properties
                .iter()
                .map(|value| parse_property(value))
                .collect::<AppResult<Vec<_>>>()?,
        }
    };

    let response: CreateInstanceResponse =
        crate::cli::request::post(port, "create", "/create", &request, 75)?;
    if json_output {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("Created {} at {}", response.class_name, response.path);
        if !response.warnings.is_empty() {
            println!("Warnings ({}):", response.warnings.len());
            for warning in response.warnings {
                println!("  - {warning}");
            }
        }
    }
    Ok(())
}

fn parse_property(value: &str) -> AppResult<CreateProperty> {
    let (name, raw) = value
        .split_once('=')
        .ok_or_else(|| AppError::Other("--property must look like Name=value".into()))?;
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::Other("--property name must not be empty".into()));
    }
    Ok(CreateProperty {
        name: name.to_string(),
        value: parse_value(raw.trim())?,
    })
}

fn parse_value(raw: &str) -> AppResult<serde_json::Value> {
    if raw.eq_ignore_ascii_case("true") {
        return Ok(serde_json::Value::Bool(true));
    }
    if raw.eq_ignore_ascii_case("false") {
        return Ok(serde_json::Value::Bool(false));
    }
    if let Some(value) = raw.strip_prefix("Vector3:") {
        let parts = parse_number_list(value, 3, "Vector3")?;
        return Ok(
            serde_json::json!({ "type": "Vector3", "x": parts[0], "y": parts[1], "z": parts[2] }),
        );
    }
    if let Some(value) = raw.strip_prefix("Color3:") {
        let parts = parse_number_list(value, 3, "Color3")?;
        return Ok(
            serde_json::json!({ "type": "Color3", "r": parts[0], "g": parts[1], "b": parts[2] }),
        );
    }
    if let Some(value) = raw.strip_prefix("UDim2:") {
        let parts = parse_number_list(value, 4, "UDim2")?;
        return Ok(serde_json::json!({
            "type": "UDim2",
            "xScale": parts[0],
            "xOffset": parts[1],
            "yScale": parts[2],
            "yOffset": parts[3]
        }));
    }
    if let Some(value) = raw.strip_prefix("Enum.") {
        let (enum_type, enum_item) = value.split_once('.').ok_or_else(|| {
            AppError::Other("enum values must look like Enum.Material.Plastic".into())
        })?;
        return Ok(
            serde_json::json!({ "type": "Enum", "enumType": enum_type, "enumItem": enum_item }),
        );
    }
    if raw.contains(',') {
        let parts = parse_number_list(raw, 3, "Vector3")?;
        return Ok(
            serde_json::json!({ "type": "Vector3", "x": parts[0], "y": parts[1], "z": parts[2] }),
        );
    }
    if let Ok(value) = raw.parse::<i64>() {
        return Ok(serde_json::Value::Number(value.into()));
    }
    if let Ok(value) = raw.parse::<f64>() {
        if let Some(number) = serde_json::Number::from_f64(value) {
            return Ok(serde_json::Value::Number(number));
        }
    }
    Ok(serde_json::Value::String(raw.to_string()))
}

fn parse_number_list(raw: &str, expected: usize, label: &str) -> AppResult<Vec<f64>> {
    let values = raw
        .split(',')
        .map(|part| {
            part.trim()
                .parse::<f64>()
                .map_err(|_| AppError::Other(format!("{label} contains a non-number")))
        })
        .collect::<AppResult<Vec<_>>>()?;
    if values.len() != expected {
        return Err(AppError::Other(format!(
            "{label} requires {expected} comma-separated numbers"
        )));
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::parse_property;

    #[test]
    fn parses_vector_property() {
        let prop = parse_property("Size=1,2,3").unwrap();
        assert_eq!(prop.name, "Size");
        assert_eq!(prop.value["type"], "Vector3");
    }

    #[test]
    fn parses_enum_property() {
        let prop = parse_property("Material=Enum.Material.Neon").unwrap();
        assert_eq!(prop.value["enumType"], "Material");
        assert_eq!(prop.value["enumItem"], "Neon");
    }
}

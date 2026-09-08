use serde_json::Value;

use crate::{
	schema::{State, document},
	validation,
};

pub fn build_configs(state: &State) -> Result<Value, String> {
	let schema = document().map_err(|e| e.to_string())?;
	if !validation::validate(state).is_empty() {
		return Err("配置尚未通过校验。".into());
	}
	schema.project(&state.value()).map_err(|e| e.to_string())
}
pub fn redact_configs(config: &Value) -> Result<Value, String> {
	document()
		.map_err(|e| e.to_string())?
		.redact(config)
		.map_err(|e| e.to_string())
}

fn escape_separators(value: String) -> String {
	value
		.replace('\u{7f}', "\\u007f")
		.replace('\u{85}', "\\u0085")
		.replace('\u{2028}', "\\u2028")
		.replace('\u{2029}', "\\u2029")
}
fn quote(value: &Value) -> String {
	escape_separators(value.to_string())
}
fn yaml(value: &Value, depth: usize, output: &mut String) {
	let pad = "  ".repeat(depth);
	match value {
		Value::Array(items) => {
			for item in items {
				if item.is_object() {
					output.push_str(&format!("{pad}-\n"));
					yaml(item, depth + 1, output);
				} else {
					output.push_str(&format!("{pad}- {}\n", quote(item)));
				}
			}
		}
		Value::Object(items) => {
			for (key, item) in items {
				let key = quote(&Value::String(key.clone()));
				if item.as_object().is_some_and(|v| !v.is_empty()) || item.as_array().is_some_and(|v| !v.is_empty()) {
					output.push_str(&format!("{pad}{key}:\n"));
					yaml(item, depth + 1, output);
				} else {
					output.push_str(&format!("{pad}{key}: {}\n", quote(item)));
				}
			}
		}
		_ => output.push_str(&format!("{pad}{}\n", quote(value))),
	}
}
pub fn serialize(config: &Value, format: &str) -> Result<String, String> {
	match format {
		"json" => serde_json::to_string_pretty(config)
			.map(|v| escape_separators(v) + "\n")
			.map_err(|_| "JSON 序列化失败。".into()),
		"toml" => toml::to_string_pretty(config).map_err(|_| "TOML 序列化失败。".into()),
		"yaml" => {
			let mut output = String::new();
			yaml(config, 0, &mut output);
			Ok(output)
		}
		_ => Err("不支持的输出格式。".into()),
	}
}

use config_generator::{dsl::Document, model::build_configs, schema::State};
use serde_json::json;

const SOURCE: &str = include_str!("../schema/example.xml");
type Result = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn unrelated_description_drives_the_complete_engine() -> Result {
	let doc = Document::parse(SOURCE)?;
	let mut state = State::new(&doc);
	assert!(doc.validate(&state.data).is_empty());
	assert_eq!(doc.ui.brand, "Notebook");
	assert_eq!(doc.ui.sections.len(), 2);
	assert_eq!(doc.exports.len(), 3);
	let config = build_configs(&doc, &state.data)?;
	assert_eq!(config["snapshot"]["items"][0]["name"], "item");
	assert_eq!(config["selected"]["item"], "item");
	assert!(config.get("audit").is_none());
	assert_eq!(doc.exports[0].command("toml"), "notebook --input snapshot.toml");
	state.data["entries"][0]["token"] = "ephemeral".into();
	let config = build_configs(&doc, &state.data)?;
	assert_eq!(doc.redact(&config)?["snapshot"]["items"][0]["secret"], "••••••••");
	assert_eq!(config["snapshot"]["items"][0]["secret"], "ephemeral");
	state.data["confirm"] = true.into();
	assert!(doc.write_field(&mut state.data, "project", "example"));
	assert_eq!(state.data["confirm"], true, "unchanged inputs do not run resets");
	assert!(doc.write_field(&mut state.data, "project", "renamed"));
	assert_eq!(state.data["confirm"], false);
	state.data["first"] = "20".into();
	assert!(doc.validate(&state.data).contains_key("last"));
	assert!(build_configs(&doc, &state.data).is_err());
	Ok(())
}

#[test]
fn arbitrary_collection_ids_selection_visibility_and_minimum() -> Result {
	let doc = Document::parse(SOURCE)?;
	let mut state = State::new(&doc);
	let first = state.rows("entries")[0];
	let second = state.add(&doc, "entries")?;
	let third = state.add(&doc, "entries")?;
	state.data["entries"][1]["id"] = "second".into();
	state.data["entries"][2]["id"] = "third".into();
	state.data["chosen"] = 2.into();
	assert!(state.remove(&doc, "entries", first));
	assert_eq!(state.data["chosen"], 1);
	assert_eq!(state.row("entries", third).and_then(|v| v["id"].as_str()), Some("third"));
	assert_eq!(state.rows("entries"), vec![second, third]);
	assert!(state.remove(&doc, "entries", third));
	assert_eq!(state.data["chosen"], 0);
	assert!(!state.remove(&doc, "entries", second));
	assert_eq!(
		state.data["entries"][0]["id"], "second",
		"the input id is not reserved for UI identity"
	);
	let last = state.add(&doc, "entries")?;
	state.data["entries"][1]["id"] = " SECOND ".into();
	assert!(doc.validate(&state.data).contains_key("entries.1.id"));
	state.data["deployment"] = "one".into();
	assert!(doc.validate(&state.data).is_empty());
	assert!(!doc.collections[0].row_visible(&doc, &state.data, 1)?);
	assert!(!state.remove(&doc, "entries", last));
	assert!(state.add(&doc, "entries").is_err());
	assert!(build_configs(&doc, &state.data)?.get("snapshot").is_none());
	state.data["chosen"] = 100.into();
	assert!(doc.validate(&state.data).contains_key("chosen"));
	Ok(())
}

#[test]
fn generation_is_declared_per_field_and_never_overwrites_other_values() -> Result {
	let doc = Document::parse(SOURCE)?;
	let fields = &doc.collections[0].fields;
	assert!(fields[0].generator.is_none());
	let uuid = fields[2].generator.as_ref().ok_or("missing generator")?;
	let value = uuid.encode(&[0; 16])?;
	let parsed = uuid::Uuid::parse_str(&value)?;
	assert_eq!(parsed.get_version_num(), 4);
	assert_eq!(parsed.get_variant(), uuid::Variant::RFC4122);
	let hex = fields[3].generator.as_ref().ok_or("missing generator")?;
	assert_eq!(hex.byte_count(), 12);
	assert_eq!(hex.encode(&[0xab; 12])?, "ab".repeat(12));
	assert!(hex.encode(&[0; 8]).is_err());
	Ok(())
}

#[test]
fn schema_extensions_fail_closed() {
	for (from, to) in [
		("rule=\"text\"", "rule=\"missing\""),
		("section=\"general\"", "section=\"missing\""),
		("selected-by=\"chosen\"", "selected-by=\"project\""),
		("all-when=\"all\"", "all-when=\"missing\""),
		("collection=\"entries\"", "collection=\"missing\""),
		("target=\"confirm\"", "target=\"missing\""),
		("mode-field=\"deployment\"", "mode-field=\"project\""),
		("generator=\"hex\" bytes=\"12\"", "generator=\"hex\" bytes=\"0\""),
		("generator=\"uuid-v4\"", "generator=\"execute\""),
		("min=\"1\" max=\"100\"", "min=\"100\" max=\"1\""),
		("op=\"gte\"", "op=\"execute\""),
		("filename=\"snapshot\"", "filename=\"../snapshot\""),
	] {
		assert!(
			Document::parse(&SOURCE.replace(from, to)).is_err(),
			"accepted invalid binding: {from}"
		);
	}
}

#[test]
fn collection_level_constraints_use_declared_error_keys() -> Result {
	let xml = SOURCE
		.replace(
			"</validators>",
			"<validator name='limit' kind='length' max='1' message='Too many items'/></validators>",
		)
		.replace(
			"</rules>",
			"<assert key='entries' message='Too many items'><valid from='/entries' rule='limit'/></assert></rules>",
		);
	let doc = Document::parse(&xml)?;
	let mut state = State::new(&doc);
	assert!(doc.validate(&state.data).is_empty());
	state.add(&doc, "entries")?;
	state.data["entries"][1]["id"] = "second".into();
	assert_eq!(
		doc.validate(&state.data).get("entries").map(String::as_str),
		Some("Too many items")
	);
	assert!(build_configs(&doc, &state.data).is_err());
	Ok(())
}

#[test]
fn malformed_state_is_not_coerced_or_exported() -> Result {
	let doc = Document::parse(SOURCE)?;
	for (field, value) in [
		("confirm", json!("false")),
		("chosen", json!(-1)),
		("entries", json!({})),
		("project", json!(null)),
		("deployment", json!("unknown")),
	] {
		let mut root = doc.defaults();
		root[field] = value;
		assert!(!doc.validate(&root).is_empty());
		assert!(build_configs(&doc, &root).is_err());
	}
	Ok(())
}

#[test]
fn production_code_contains_no_product_identifiers() {
	for source in [
		include_str!("../src/app.rs"),
		include_str!("../src/schema.rs"),
		include_str!("../src/validation.rs"),
		include_str!("../src/model.rs"),
		include_str!("../src/dsl.rs"),
		include_str!("../src/dsl/parser.rs"),
		include_str!("../src/dsl/metadata.rs"),
		include_str!("../src/dsl/rules.rs"),
		include_str!("../src/dsl/wire.rs"),
		include_str!("../index.html"),
		include_str!("../build.rs"),
		include_str!("../Cargo.toml"),
	] {
		for name in [
			"tuic",
			"quinn",
			"socks",
			"acme",
			"tlsMode",
			"activeUser",
			"forwards",
			"maxBackoff",
		] {
			assert!(
				!source.to_lowercase().contains(&name.to_lowercase()),
				"product identifier {name} leaked into engine"
			);
		}
	}
}

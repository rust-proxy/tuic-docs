use config_generator::{
	dsl::Document,
	session::{Action, Session},
};
use serde_json::{Value, json};

type Result = std::result::Result<(), Box<dyn std::error::Error>>;
const XML: &str = include_str!("../schema/example.xml");

fn random(bytes: &mut [u8]) -> std::result::Result<(), String> {
	bytes.fill(0xab);
	Ok(())
}
fn action(session: &mut Session, value: Value) -> Result {
	session.dispatch(serde_json::from_value(value)?, &mut random)?;
	Ok(())
}

#[test]
fn wire_actions_preserve_row_identity_selection_and_business_id() -> Result {
	let mut s = Session::new(Document::parse(XML)?);
	s.initialize(&mut random)?;
	let first = s.snapshot("", false).sections[1].collections[0].rows[0].id.clone();
	action(&mut s, json!({"type":"add", "collection":"entries"}))?;
	let second = s.snapshot("", false).sections[1].collections[0].rows[1].id.clone();
	action(
		&mut s,
		json!({"type":"set-row", "collection":"entries", "id":second, "field":"id", "value":"kept"}),
	)?;
	action(&mut s, json!({"type":"set", "field":"chosen", "value":"1"}))?;
	action(&mut s, json!({"type":"remove", "collection":"entries", "id":first}))?;
	let view = s.snapshot("selected", false);
	let collection = &view.sections[1].collections[0];
	assert_eq!(collection.rows[0].id, second);
	assert_eq!(collection.rows[0].fields[0].value, "kept");
	assert_eq!(collection.rows[0].fields[0].path, "entries.0.id");
	assert_eq!(collection.selector.as_ref().ok_or("missing selector")?.value, "0");
	assert!(!collection.removable);
	assert_eq!(serde_json::from_str::<Value>(&view.preview)?["item"], "kept");
	// Stale DOM events cannot silently write to the remaining row.
	let before = serde_json::to_value(s.snapshot("", true))?;
	assert!(
		action(
			&mut s,
			json!({"type":"set-row", "collection":"entries", "id":first, "field":"id", "value":"stale"})
		)
		.is_err()
	);
	assert!(action(&mut s, json!({"type":"remove", "collection":"entries", "id":second})).is_err());
	assert_eq!(serde_json::to_value(s.snapshot("", true))?, before);
	Ok(())
}

#[test]
fn preview_export_resets_and_conditional_output_share_one_engine() -> Result {
	let mut s = Session::new(Document::parse(XML)?);
	s.initialize(&mut random)?;
	let view = s.snapshot("snapshot", false);
	assert_eq!(
		serde_json::from_str::<Value>(&view.preview)?["items"][0]["secret"],
		"••••••••"
	);
	let file = s.export("snapshot")?;
	assert_eq!(file.filename, "snapshot.json");
	assert_eq!(
		serde_json::from_str::<Value>(&file.text)?["items"][0]["secret"],
		"ab".repeat(12)
	);
	action(&mut s, json!({"type":"set", "field":"confirm", "value":"true"}))?;
	assert_eq!(s.snapshot("audit", false).selected, "audit");
	action(&mut s, json!({"type":"set", "field":"project", "value":"example"}))?;
	assert_eq!(s.snapshot("audit", false).selected, "audit", "unchanged writes do not reset");
	action(&mut s, json!({"type":"set", "field":"project", "value":"changed"}))?;
	assert_eq!(s.snapshot("audit", false).selected, "snapshot");
	action(&mut s, json!({"type":"set", "field":"confirm", "value":"true"}))?;
	action(&mut s, json!({"type":"set", "field":"seed", "value":"manual"}))?;
	action(&mut s, json!({"type":"set", "field":"confirm", "value":"true"}))?;
	action(&mut s, json!({"type":"generate", "field":"seed"}))?;
	assert_eq!(
		s.snapshot("audit", false).selected,
		"snapshot",
		"generated writes apply resets"
	);
	action(&mut s, json!({"type":"set", "field":"first", "value":"50"}))?;
	let view = s.snapshot("", true);
	assert!(!view.valid);
	assert!(view.errors.contains_key("last"));
	assert!(s.export("").is_err());
	Ok(())
}

#[test]
fn failed_randomness_and_invalid_actions_do_not_partially_mutate_state() -> Result {
	let mut s = Session::new(Document::parse(XML)?);
	let before = serde_json::to_value(s.snapshot("", true))?;
	let mut calls = 0;
	let mut fail = move |bytes: &mut [u8]| {
		calls += 1;
		if calls > 1 {
			return Err("Random unavailable".into());
		}
		random(bytes)
	};
	assert!(s.initialize(&mut fail).is_err());
	assert_eq!(serde_json::to_value(s.snapshot("", true))?, before);
	assert!(
		s.dispatch(
			Action::Add {
				collection: "entries".into()
			},
			&mut fail
		)
		.is_err()
	);
	assert_eq!(serde_json::to_value(s.snapshot("", true))?, before);
	for value in [
		json!({"type":"set", "field":"confirm", "value":"not a boolean"}),
		json!({"type":"set", "field":"missing", "value":"value"}),
		json!({"type":"set-row", "collection":"entries", "id":"bad", "field":"id", "value":"value"}),
	] {
		assert!(action(&mut s, value).is_err());
	}
	assert!(serde_json::from_value::<Action>(json!({"type":"set", "field":"project", "value":"value", "extra":true})).is_err());
	assert_eq!(serde_json::to_value(s.snapshot("", true))?, before);
	Ok(())
}

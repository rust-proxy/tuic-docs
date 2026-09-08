use config_generator::{dsl::Document, schema::State};
use serde_json::json;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
fn description(inputs: &str, definitions: &str, outputs: &str) -> String {
	format!(
		r#"<config-dsl version="4" target-version="test"><inputs>{inputs}</inputs>{definitions}<outputs>{outputs}</outputs></config-dsl>"#
	)
}

#[test]
fn xml_fields_defaults_entities_and_binding() -> Result {
	let source = description(
		r#"
 <!-- Static metadata; no executable expressions. -->
 <field name='title' type='string' default='&lt;demo&gt; &amp; &quot;x&quot; &apos;y&apos; &#20013;&#x6587;' label='标题'/>
 <field name='enabled' type='boolean' default='false' label='开启'/>
 <field name='count' type='integer' default='0' label='数量'/>
 <field name='mode' type='enum' default='b' label='模式'><option value='a' label='A'/><option value='b' label='B'/></field>
 <collection name='rows' initial-items='2'><field name='value' type='string' default='' label='值'/></collection>
 "#,
		"",
		"",
	);
	let doc = Document::parse(&source)?;
	let mut defaults = doc.defaults();
	assert_eq!(defaults["title"], "<demo> & \"x\" 'y' 中文");
	assert_eq!(defaults["enabled"], false);
	assert_eq!(defaults["count"], 0);
	assert_eq!(defaults["mode"], "b");
	assert_eq!(defaults["rows"].as_array().map(Vec::len), Some(2));
	assert!(!doc.fields[1].write_value(&mut defaults, "1"));
	assert!(doc.fields[1].write_value(&mut defaults, "true"));
	assert_eq!(defaults["enabled"], true);
	assert!(doc.fields[0].write_value(&mut defaults, "<script>not executed</script>"));
	assert_eq!(doc.fields[0].read_value(&defaults), "<script>not executed</script>");
	Ok(())
}

#[test]
fn static_conditions_preserve_false_zero_and_empty_values() -> Result {
	let source = description(
		r#"<field name="unused" type="string" default="" label="unused"/>"#,
		r#"<conditions><condition name="never"><not><all><eq from="/unused" value=""/></all></not></condition></conditions>"#,
		r#"<boolean name="flag" value="false"/><integer name="zero" value="0"/><string name="empty" value=""/>
 <object name="hidden" when="never"><string name="unused" from="/unused" transform="integer"/></object>"#,
	);
	let doc = Document::parse(&source)?;
	assert_eq!(doc.project(&doc.defaults())?, json!({"flag":false,"zero":0,"empty":""}));
	Ok(())
}

#[test]
fn collections_filter_select_normalize_and_redact() -> Result {
	let source = description(
		r#"
 <field name="index" type="integer" default="1" label="选择"/>
 <collection name="rows" initial-items="0">
 <field name="key" type="string" default="" label="键"/>
 <field name="password" type="string" default="" label="密码"/>
 <field name="kind" type="string" default="tcp" label="类型"/>
 </collection>"#,
		"",
		r#"
 <record name="users" from="/rows" key="key" key-transform="trim lowercase"><string from="password" secret="true"/></record>
 <string name="selected" secret="true"><select from="/rows" index="/index"><source from="password"/></select></string>
 <list name="tcp" from="/rows" where-field="kind" equals="tcp" omit-empty="true"><object><string name="name" from="key" transform="trim"/></object></list>
 <list name="empty" from="/rows" where-field="kind" equals="unused"><string from="key"/></list>
 <list name="omitted" from="/rows" where-field="kind" equals="unused" omit-empty="true"><string from="key"/></list>"#,
	);
	let doc = Document::parse(&source)?;
	let mut state =
		json!({"index":1,"rows":[{"key":" A ","password":" one ","kind":"tcp"},{"key":"B","password":"two","kind":"udp"}]});
	let config = doc.project(&state)?;
	assert_eq!(config["users"]["a"], " one ");
	assert_eq!(config["selected"], "two");
	assert_eq!(config["tcp"], json!([{"name":"A"}]));
	assert_eq!(config["empty"], json!([]));
	assert!(config.get("omitted").is_none());
	let redacted = doc.redact(&config)?;
	assert_eq!(redacted["users"]["a"], "••••••••");
	assert_eq!(redacted["selected"], "••••••••");
	assert_eq!(config["selected"], "two");
	assert!(doc.redact(&json!({"unknown":"value"})).is_err());
	assert!(doc.redact(&json!({"users":["wrong"]})).is_err());
	state["rows"][1]["key"] = "a".into();
	assert!(doc.project(&state).is_err());
	state["rows"][1]["key"] = "b".into();
	state["index"] = 9.into();
	assert!(doc.project(&state).is_err());
	state["index"] = "0".into();
	assert!(doc.project(&state).is_err());
	Ok(())
}

#[test]
fn named_values_conditions_and_ipv6_endpoint() -> Result {
	let source = description(
		r#"
 <field name="host" type="string" default=" [2001:db8::1] " label="主机"/>
 <field name="port" type="string" default="0443" label="端口"/>
 <field name="override" type="string" default="" label="覆盖"/>
 <field name="enabled" type="boolean" default="false" label="开启"/>"#,
		r#"<conditions>
 <condition name="enabled"><truthy from="/enabled"/></condition>
 <condition name="address"><any><truthy from="/enabled"/><ip ref="host"/></any></condition>
 </conditions><values>
 <value name="host"><coalesce><source from="/override" when="enabled"/><source from="/host" transform="trim unbracket"/></coalesce></value>
 </values>"#,
		r#"<string name="address" when="address"><endpoint><source ref="host"/><source from="/port" transform="integer"/></endpoint></string>"#,
	);
	let doc = Document::parse(&source)?;
	assert_eq!(doc.project(&doc.defaults())?["address"], "[2001:db8::1]:443");
	Ok(())
}

#[test]
fn syntax_and_semantic_errors_are_rejected() {
	for invalid in [
		"<config-dsl>",
		"<config-dsl version='4' target-version='test'><inputs></outputs></config-dsl>",
		"<!DOCTYPE x SYSTEM 'file:///never-read'><config-dsl/>",
		"<?xml version='1.0'?><config-dsl/>",
		"<config-dsl version='4' version='4'/>",
		"<!-- invalid -- comment --><config-dsl/>",
		"<config-dsl version='4' target-version='test'>text<inputs/><outputs/></config-dsl>",
	] {
		assert!(Document::parse(invalid).is_err());
	}
	for inputs in [
		"<field name='x' type='boolean' default='yes' label='x'/>",
		"<field name='x' type='string' default='&external;' label='x'/>",
		"<field name='x' type='string' default='&#0;' label='x'/>",
		"<field name='x' type='string' default='&#xD800;' label='x'/>",
		"<field name='x' type='enum' default='other' label='x'><option value='a' label='A'/></field>",
		"<field name='x' type='string' default='' label='x' rule='execute'/>",
		"<field name='x' type='string' default='' label='x' oninput='script()'/>",
		"<collection name='x' initial-items='1001'/>",
		"<field name='x' type='string' default='' label='x'/><field name='x' type='string' default='' label='x'/>",
	] {
		assert!(Document::parse(&description(inputs, "", "")).is_err());
	}
	for output in [
		"<script name='x'/>",
		"<string name='x' value='a' from='/x'/>",
		"<string name='x' value='a'/><string name='x' value='b'/>",
		"<string name='x' value='a' when='missing'/>",
		"<string name='x' ref='missing'/>",
		"<string name='x' value='a' transform='|s| s.trim()'/>",
		"<string name='x' value='a' secret='yes'/>",
		"<string name='x' value='a' unit='script'/>",
		"<boolean name='x' value='false()'/>",
		"<string name='x' from='/missing'/>",
		"<string name='x' from='/x/../x'/>",
		"<list name='x' from='/x' where-field='x'><string from='x'/></list>",
		"<enum name='x' value='a' options='x'/>",
	] {
		let source = description("<field name='x' type='string' default='' label='x'/>", "", output);
		assert!(Document::parse(&source).is_err());
	}
	assert!(Document::parse(&(description("", "", "") + "trailing")).is_err());
}

#[test]
fn reject_cycles_including_cross_kind_and_unused_definitions() {
	for definitions in [
		"<values><value name='a'><source ref='a'/></value></values>",
		"<values><value name='a'><source ref='b'/></value><value name='b'><source ref='a'/></value></values>",
		"<conditions><condition name='a'><use ref='a'/></condition></conditions>",
		"<conditions><condition name='a'><ip ref='b'/></condition></conditions><values><value name='b'><source from='/x' when='a'/></value></values>",
	] {
		let source = description("<field name='x' type='string' default='' label='x'/>", definitions, "");
		let error = Document::parse(&source).err().map(|e| e.to_string()).unwrap_or_default();
		assert!(error.contains("循环依赖"));
	}
}

#[test]
fn diagnostics_have_location_and_never_include_field_values() -> Result {
	let bad = "<config-dsl version='4' target-version='test'>\n<inputs/>\n<outputs><string name='token' value='never-log-this' unknown='true'/></outputs></config-dsl>";
	let error = Document::parse(bad).err().ok_or("expected error")?.to_string();
	assert!(error.contains("3:") && !error.contains("never-log-this"));
	let source = description(
		"<field name='token' type='string' default='' label='token'/>",
		"",
		"<object name='nested'><integer name='token' from='/token' secret='true'/></object>",
	);
	let doc = Document::parse(&source)?;
	let error = doc
		.project(&json!({"token":"never-log-this"}))
		.err()
		.ok_or("expected error")?
		.to_string();
	assert!(error.contains("nested.token") && !error.contains("never-log-this"));
	assert!(doc.project(&json!({})).is_err());
	Ok(())
}

#[test]
fn extending_static_xml_changes_defaults_forms_and_projection_without_rust_code() -> Result {
	let xml = include_str!("../schema/example.xml")
		.replace(
			"</inputs>",
			r#"<field name="note" type="string" default="new default" section="general" label="备注"/></inputs>"#,
		)
		.replace("<outputs>", r#"<outputs><string name="note" from="/note"/>"#);
	let doc = Document::parse(&xml)?;
	let mut state = State::new(&doc);
	let field = doc.fields.iter().find(|f| f.key == "note").ok_or("field missing")?;
	assert_eq!(field.read_value(&state.data), "new default");
	assert!(doc.write_field(&mut state.data, "note", "updated via metadata"));
	assert_eq!(doc.project(&state.data)?["note"], "updated via metadata");
	Ok(())
}

#[test]
fn form_binding_preserves_stable_row_ids() -> Result {
	let doc = Document::parse(include_str!("../schema/example.xml"))?;
	let mut state = State::new(&doc);
	let id = state.add(&doc, "entries")?;
	let ids = state.rows("entries");
	for field in &doc.fields {
		assert!(doc.write_field(&mut state.data, &field.key, &field.read_value(&doc.defaults())));
	}
	assert_eq!(state.rows("entries"), ids);
	assert_eq!(state.row("entries", id).map(|row| row["id"].clone()), Some(json!("item")));
	assert!(state.data.get("_id").is_none());
	Ok(())
}

#[test]
fn condition_types_and_document_limits() -> Result {
	let xml = description(
		"<field name='x' type='boolean' default='false' label='x'/>",
		"<conditions><condition name='test'><truthy from='/x'/></condition><condition name='ip'><ip from='/x'/></condition><condition name='eq'><eq from='/x' value=''/></condition></conditions>",
		"",
	);
	let doc = Document::parse(&xml)?;
	for (name, value) in [
		("test", json!({"x":"false"})),
		("ip", json!({"x":false})),
		("eq", json!({"x":[]})),
	] {
		assert!(doc.condition(name, &value, &value).is_err());
	}
	let deep = description(
		"",
		"",
		&format!(
			"{}<string name='v' value='ok'/>{}",
			"<object name='o'>".repeat(65),
			"</object>".repeat(65)
		),
	);
	assert!(Document::parse(&deep).is_err());
	assert!(Document::parse(&" ".repeat(1_048_577)).is_err());
	Ok(())
}

#[test]
fn serde_preserves_attribute_whitespace_and_decodes_entities_once() -> Result {
	let value = " \tline one\r\nline two\r &#9;&#10;&#13;&amp;#10;&amp;lt; ";
	let xml = description(
		&format!("<field name='text' type='string' default='{value}' label='文本'/>"),
		"",
		"<string name='text' from='/text'/>",
	);
	let doc = Document::parse(&xml)?;
	assert_eq!(
		doc.project(&doc.defaults())?["text"],
		" \tline one\r\nline two\r \t\n\r&#10;&lt; "
	);
	Ok(())
}

#[test]
fn serde_preserves_interleaved_children_and_empty_attributes() -> Result {
	let xml = description(
		"<field name='a' type='string' default='' label=''/><collection name='rows' initial-items='0'><field name='v' type='string' default='' label=''/></collection><field name='b' type='string' default='b' label='b'/>",
		"<values><value name='pick'><coalesce><source from='/a'/><source from='/b'/><source from='/a'/></coalesce></value></values>",
		"<object name='first'/><string name='picked' ref='pick'/><object name='last'/>",
	);
	let doc = Document::parse(&xml)?;
	assert_eq!(
		doc.fields.iter().map(|field| field.key.as_str()).collect::<Vec<_>>(),
		["a", "b"]
	);
	let result = doc.project(&doc.defaults())?;
	assert_eq!(result["picked"], "b");
	assert_eq!(
		result
			.as_object()
			.ok_or("object")?
			.keys()
			.map(String::as_str)
			.collect::<Vec<_>>(),
		["first", "picked", "last"]
	);
	Ok(())
}

#[test]
fn xml_subset_is_checked_before_serde() {
	let valid = description("", "", "");
	for invalid in [format!("{valid}{valid}"), format!("\u{feff}{valid}")] {
		assert!(Document::parse(&invalid).is_err());
	}
	for content in [
		"<![CDATA[]]>",
		"&#32;",
		"<?processing instruction?>",
		"<x:field/>",
		"<field xmlns='urn:test'/>",
		"<field xmlns:x='urn:test'/>",
		"<field name='a'name='b'/>",
		"<field name='a' type='string' default='x'label='a'/>",
		"<field name='a' type='string' default='' label='a' xmlns='urn:test'/>",
		"<field name='a' label='<raw'/>",
		"<field name='a' label='&#xFFFE;'/>",
		"<!-- bad --->",
		"< field/>",
		"<field name='a' label='&#+65;'/>",
		"<field name='a' label='&#x+41;'/>",
		"<field></ field>",
		"<field></field extra>",
	] {
		assert!(
			Document::parse(&description(content, "", "")).is_err(),
			"accepted invalid XML subset"
		);
	}
	assert!(Document::parse(&valid.replace("config-dsl", "object")).is_err());
	let deep = format!("{}{}", "<object>".repeat(4000), "</object>".repeat(4000));
	assert!(
		Document::parse(&deep)
			.err()
			.is_some_and(|error| error.to_string().contains("64"))
	);
}

#[test]
fn maximum_xml_depth_remains_supported() -> Result {
	let xml = description(
		"",
		"",
		&format!(
			"{}<string name='v' value='ok'/>{}",
			"<object name='o'>".repeat(62),
			"</object>".repeat(62)
		),
	);
	Document::parse(&xml)?;
	assert!(
		Document::parse(&xml.replace(
			"<string name='v' value='ok'/>",
			"<object name='v'><string name='v' value='ok'/></object>"
		))
		.is_err()
	);
	Ok(())
}

use pest::{Parser, error::LineColLocation, iterators::Pair};

use super::*;

#[derive(pest_derive::Parser)]
#[grammar = "dsl/xml.pest"]
struct XmlParser;

pub(super) fn parse(source: &str) -> Result<Document, DslError> {
	if source.len() > 1_048_576 {
		return Err(DslError("描述文件超过 1 MiB".into()));
	}
	if !source.chars().all(xml_char) {
		return Err(DslError("描述包含无效 XML 字符".into()));
	}
	let mut pairs = XmlParser::parse(Rule::document, source).map_err(|e| {
		let (line, column) = match e.line_col {
			LineColLocation::Pos(p) | LineColLocation::Span(p, _) => p,
		};
		DslError(format!("{line}:{column} XML 语法错误"))
	})?;
	let document = pairs.next().ok_or_else(|| DslError("缺少文档".into()))?;
	let root_pair = document
		.into_inner()
		.find(|p| p.as_rule() == Rule::element)
		.ok_or_else(|| DslError("缺少根元素".into()))?;
	let root = element(root_pair, 0)?;
	if root.tag != "config-dsl" {
		return Err(root.error("根元素必须为 config-dsl"));
	}
	attrs(&root, &["version", "target-version"])?;
	if root.required("version")? != "3" {
		return Err(root.error("仅支持 DSL version=3"));
	}
	let mut sections = BTreeMap::new();
	for child in &root.children {
		if !["inputs", "conditions", "values", "outputs"].contains(&child.tag.as_str()) {
			return Err(child.error("未知文档区块"));
		}
		attrs(child, &[])?;
		if sections.insert(child.tag.as_str(), child).is_some() {
			return Err(child.error("重复文档区块"));
		}
	}
	let inputs = sections.get("inputs").ok_or_else(|| root.error("缺少 inputs"))?;
	let outputs = sections.get("outputs").ok_or_else(|| root.error("缺少 outputs"))?;
	let mut fields = Vec::new();
	let mut collections = Vec::new();
	let mut names = BTreeSet::new();
	for node in &inputs.children {
		let name = identifier(node, "name")?;
		if !names.insert(name.to_owned()) {
			return Err(node.error("重复输入名称"));
		}
		match node.tag.as_str() {
			"field" => fields.push(field(node)?),
			"collection" => {
				attrs(node, &["name", "initial-items", "when"])?;
				let initial_items = node
					.required("initial-items")?
					.parse::<usize>()
					.ok()
					.filter(|n| *n <= 1000)
					.ok_or_else(|| node.error("initial-items 必须为 0..1000"))?;
				let mut items = Vec::new();
				let mut keys = BTreeSet::new();
				for child in &node.children {
					let f = field(child)?;
					if !keys.insert(f.key.clone()) {
						return Err(child.error("重复集合字段"));
					}
					items.push(f);
				}
				if items.is_empty() {
					return Err(node.error("集合必须声明字段"));
				}
				collections.push(Collection {
					name: name.into(),
					fields: items,
					initial_items,
					when: node.attr("when").map(str::to_owned),
				});
			}
			_ => return Err(node.error("inputs 只接受 field 或 collection")),
		}
	}
	let mut conditions = BTreeMap::new();
	let mut values = BTreeMap::new();
	for (section, tag, definitions) in [("conditions", "condition", &mut conditions), ("values", "value", &mut values)] {
		if let Some(section) = sections.get(section) {
			for node in &section.children {
				if node.tag != tag {
					return Err(node.error("未知定义元素"));
				}
				attrs(node, &["name"])?;
				let name = identifier(node, "name")?;
				let body = node.single()?;
				if tag == "condition" {
					condition(body)?;
				} else {
					expression(body)?;
				}
				if definitions.insert(name.into(), body.clone()).is_some() {
					return Err(node.error("重复定义名称"));
				}
			}
		}
	}
	output(outputs, false)?;
	let doc = Document {
		target_version: root.required("target-version")?.into(),
		fields,
		collections,
		conditions,
		values,
		outputs: (*outputs).clone(),
	};
	check_references(&root, &doc)?;
	let mut done = BTreeSet::new();
	for (prefix, defs) in [("condition", &doc.conditions), ("value", &doc.values)] {
		for name in defs.keys() {
			visit(&format!("{prefix}:{name}"), &doc, &mut BTreeSet::new(), &mut done)?;
		}
	}
	Ok(doc)
}
fn element(pair: Pair<'_, Rule>, depth: usize) -> Result<Element, DslError> {
	let (line, column) = pair.as_span().start_pos().line_col();
	if depth > 64 {
		return Err(DslError(format!("{line}:{column} 元素嵌套超过 64 层")));
	}
	let pair = if pair.as_rule() == Rule::element {
		pair.into_inner().next().ok_or_else(|| DslError("缺少元素".into()))?
	} else {
		pair
	};
	let mut result = Element {
		tag: String::new(),
		attrs: BTreeMap::new(),
		children: Vec::new(),
		line,
		column,
	};
	for part in pair.into_inner() {
		match part.as_rule() {
			Rule::name => {
				if result.tag.is_empty() {
					result.tag = part.as_str().into();
				} else if result.tag != part.as_str() {
					return Err(result.error("开始与结束标签不匹配"));
				}
			}
			Rule::attribute => {
				let mut parts = part.into_inner();
				let name = parts.next().ok_or_else(|| result.error("缺少属性名"))?;
				let quoted = parts.next().ok_or_else(|| result.error("缺少属性值"))?;
				let text = quoted.as_str();
				let value = decode(&result, &text[1..text.len() - 1])?;
				if result.attrs.insert(name.as_str().into(), value).is_some() {
					return Err(result.error("重复属性"));
				}
			}
			Rule::element => result.children.push(element(part, depth + 1)?),
			_ => {}
		}
	}
	Ok(result)
}
fn xml_char(c: char) -> bool {
	matches!(c as u32, 0x9 | 0xa | 0xd | 0x20..=0xd7ff | 0xe000..=0xfffd | 0x10000..=0x10ffff)
}
fn decode(node: &Element, text: &str) -> Result<String, DslError> {
	let mut result = String::new();
	let mut rest = text;
	while let Some(i) = rest.find('&') {
		result.push_str(&rest[..i]);
		rest = &rest[i + 1..];
		let end = rest.find(';').ok_or_else(|| node.error("实体缺少分号"))?;
		let entity = &rest[..end];
		let ch = match entity {
			"amp" => Some('&'),
			"lt" => Some('<'),
			"gt" => Some('>'),
			"quot" => Some('"'),
			"apos" => Some('\''),
			_ => {
				let number = if let Some(hex) = entity.strip_prefix("#x") {
					u32::from_str_radix(hex, 16).ok()
				} else {
					entity.strip_prefix('#').and_then(|n| n.parse::<u32>().ok())
				};
				number.and_then(char::from_u32)
			}
		}
		.filter(|c| xml_char(*c))
		.ok_or_else(|| node.error("不支持的实体或字符引用"))?;
		result.push(ch);
		rest = &rest[end + 1..];
	}
	result.push_str(rest);
	if !result.chars().all(xml_char) {
		return Err(node.error("无效 XML 字符"));
	}
	Ok(result)
}
fn attrs(node: &Element, allowed: &[&str]) -> Result<(), DslError> {
	if node.attrs.keys().any(|k| !allowed.contains(&k.as_str())) {
		return Err(node.error("含有未知属性"));
	}
	for flag in ["secret", "omit-empty"] {
		if let Some(value) = node.attr(flag)
			&& !["true", "false"].contains(&value)
		{
			return Err(node.error("布尔属性只能为 true 或 false"));
		}
	}
	for attr in ["transform", "key-transform"] {
		if let Some(value) = node.attr(attr)
			&& (value.is_empty()
				|| value
					.split_whitespace()
					.any(|s| !["trim", "lowercase", "unbracket", "integer"].contains(&s)))
		{
			return Err(node.error("未知转换，不能使用脚本表达式"));
		}
	}
	if let Some(unit) = node.attr("unit")
		&& !["s", "ms"].contains(&unit)
	{
		return Err(node.error("不支持的时间单位"));
	}
	Ok(())
}
fn identifier<'a>(node: &'a Element, attr: &str) -> Result<&'a str, DslError> {
	let value = node.required(attr)?;
	if value.is_empty()
		|| !value.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
		|| value.as_bytes()[0].is_ascii_digit()
	{
		return Err(node.error("名称必须是静态标识符"));
	}
	Ok(value)
}
fn empty(node: &Element) -> Result<(), DslError> {
	if node.children.is_empty() {
		Ok(())
	} else {
		Err(node.error("不接受子元素"))
	}
}
fn source(node: &Element) -> Result<(), DslError> {
	if usize::from(node.attr("from").is_some()) + usize::from(node.attr("ref").is_some()) != 1 {
		return Err(node.error("必须指定且只能指定 from 或 ref"));
	}
	Ok(())
}
fn field(node: &Element) -> Result<InputField, DslError> {
	if node.tag != "field" {
		return Err(node.error("必须为 field 元素"));
	}
	attrs(
		node,
		&[
			"name",
			"type",
			"default",
			"label",
			"placeholder",
			"hint",
			"section",
			"widget",
			"when",
			"rule",
		],
	)?;
	let key = identifier(node, "name")?.into();
	let mut options = Vec::new();
	let mut seen = BTreeSet::new();
	for child in &node.children {
		if child.tag != "option" {
			return Err(child.error("field 只接受 option"));
		}
		attrs(child, &["value", "label"])?;
		empty(child)?;
		let value = child.required("value")?;
		if !seen.insert(value) {
			return Err(child.error("重复枚举值"));
		}
		options.push((value.into(), child.required("label")?.into()));
	}
	let raw = node.required("default")?;
	let (default, mut kind) = match node.required("type")? {
		"string" => (Value::String(raw.into()), InputKind::Text),
		"boolean" => (
			raw.parse::<bool>()
				.map(Value::Bool)
				.map_err(|_| node.error("默认值必须为布尔值"))?,
			InputKind::Toggle,
		),
		"integer" => (
			raw.parse::<u64>()
				.map(Value::from)
				.map_err(|_| node.error("默认值必须为非负整数"))?,
			InputKind::Number,
		),
		"enum" => {
			if !options.iter().any(|(v, _)| v == raw) {
				return Err(node.error("默认值不在枚举中"));
			}
			(Value::String(raw.into()), InputKind::Select)
		}
		_ => return Err(node.error("未知输入类型")),
	};
	if kind != InputKind::Select && !options.is_empty() {
		return Err(node.error("只有 enum 接受 option"));
	}
	if let Some(widget) = node.attr("widget") {
		if node.attr("type") != Some("string") {
			return Err(node.error("widget 仅用于 string 输入"));
		}
		kind = match widget {
			"text" => InputKind::Text,
			"number" => InputKind::Number,
			"password" => InputKind::Password,
			"email" => InputKind::Email,
			_ => return Err(node.error("未知控件类型")),
		};
	}
	let rule = node.attr("rule").unwrap_or_default();
	if ![
		"",
		"required",
		"host",
		"port",
		"socket",
		"email",
		"socks-credential",
		"milliseconds",
		"uuid",
		"password",
		"endpoint",
		"seconds",
	]
	.contains(&rule)
	{
		return Err(node.error("未知输入校验规则"));
	}
	Ok(InputField {
		key,
		label: node.required("label")?.into(),
		placeholder: node.attr("placeholder").unwrap_or_default().into(),
		hint: node.attr("hint").unwrap_or_default().into(),
		section: node.attr("section").unwrap_or_default().into(),
		kind,
		options,
		rule: rule.into(),
		default,
		when: node.attr("when").map(str::to_owned),
	})
}
fn condition(node: &Element) -> Result<(), DslError> {
	match node.tag.as_str() {
		"all" | "any" | "not" => {
			attrs(node, &[])?;
			if node.children.is_empty() {
				return Err(node.error("条件组不能为空"));
			}
			if node.tag == "not" {
				node.single()?;
			}
			for child in &node.children {
				condition(child)?;
			}
		}
		"use" => {
			attrs(node, &["ref"])?;
			identifier(node, "ref")?;
			empty(node)?;
		}
		"eq" | "truthy" | "ip" => {
			attrs(
				node,
				if node.tag == "eq" {
					&["from", "ref", "value"]
				} else {
					&["from", "ref"]
				},
			)?;
			source(node)?;
			empty(node)?;
			if node.tag == "eq" {
				node.required("value")?;
			}
		}
		_ => return Err(node.error("未知条件元素")),
	}
	Ok(())
}
fn expression(node: &Element) -> Result<(), DslError> {
	match node.tag.as_str() {
		"source" => {
			attrs(node, &["from", "ref", "transform", "when"])?;
			source(node)?;
			empty(node)?;
		}
		"coalesce" | "endpoint" => {
			attrs(node, &["when"])?;
			if node.children.is_empty() || (node.tag == "endpoint" && node.children.len() != 2) {
				return Err(node.error("值元素的子元素数量错误"));
			}
			for child in &node.children {
				expression(child)?;
			}
		}
		"select" => {
			attrs(node, &["from", "index", "when"])?;
			node.required("from")?;
			node.required("index")?;
			expression(node.single()?)?;
		}
		_ => return Err(node.error("未知值元素，不能使用编程语言表达式")),
	}
	Ok(())
}
fn output(node: &Element, named: bool) -> Result<(), DslError> {
	if named {
		identifier(node, "name")?;
	} else if node.tag != "outputs" && node.attr("name").is_some() {
		return Err(node.error("集合项不能命名"));
	}
	match node.tag.as_str() {
		"outputs" | "object" => {
			attrs(
				node,
				if node.tag == "outputs" {
					&[]
				} else {
					&["name", "when", "secret"]
				},
			)?;
			let mut names = BTreeSet::new();
			for child in &node.children {
				output(child, true)?;
				if !names.insert(child.required("name")?) {
					return Err(child.error("重复输出字段"));
				}
			}
		}
		"list" | "record" => {
			attrs(
				node,
				if node.tag == "record" {
					&["name", "when", "secret", "from", "key", "key-transform", "omit-empty"]
				} else {
					&["name", "when", "secret", "from", "where-field", "equals", "omit-empty"]
				},
			)?;
			if node.tag == "record" {
				node.required("from")?;
				node.required("key")?;
			}
			if node.attr("where-field").is_some() != node.attr("equals").is_some()
				|| (node.attr("where-field").is_some() && node.attr("from").is_none())
			{
				return Err(node.error("列表过滤需要 from、where-field 和 equals"));
			}
			output(node.single()?, false)?;
		}
		"string" | "boolean" | "integer" | "enum" => {
			attrs(
				node,
				&[
					"name",
					"when",
					"secret",
					"from",
					"ref",
					"value",
					"transform",
					"unit",
					"options",
				],
			)?;
			let count = ["from", "ref", "value"].iter().filter(|a| node.attr(a).is_some()).count()
				+ usize::from(!node.children.is_empty());
			if count != 1 {
				return Err(node.error("标量必须有一个来源、常量或值元素"));
			}
			if !node.children.is_empty() {
				expression(node.single()?)?;
			}
			if node.tag == "enum" {
				node.required("options")?;
			} else if node.attr("options").is_some() {
				return Err(node.error("options 只用于 enum"));
			}
			if let Some(value) = node.attr("value")
				&& ((node.tag == "boolean" && value.parse::<bool>().is_err())
					|| (node.tag == "integer" && value.parse::<i64>().is_err()))
			{
				return Err(node.error("常量与输出类型不匹配"));
			}
			if node.tag != "string" && node.attr("unit").is_some() {
				return Err(node.error("unit 只用于 string 输出"));
			}
		}
		_ => return Err(node.error("未知输出元素")),
	}
	Ok(())
}
fn references(node: &Element, result: &mut BTreeSet<String>) {
	if let Some(name) = node.attr("when") {
		result.insert(format!("condition:{name}"));
	}
	if let Some(name) = node.attr("ref") {
		result.insert(format!("{}:{name}", if node.tag == "use" { "condition" } else { "value" }));
	}
	for child in &node.children {
		references(child, result);
	}
}
fn check_references(node: &Element, doc: &Document) -> Result<(), DslError> {
	if let Some(name) = node.attr("when")
		&& !doc.conditions.contains_key(name)
	{
		return Err(node.error("引用了未声明的条件"));
	}
	if let Some(name) = node.attr("ref") {
		let defs = if node.tag == "use" { &doc.conditions } else { &doc.values };
		if !defs.contains_key(name) {
			return Err(node.error("引用了未声明的定义"));
		}
	}
	if let Some(name) = node.attr("options")
		&& !doc.fields.iter().any(|f| f.key == name && f.kind == InputKind::Select)
	{
		return Err(node.error("options 必须引用输入枚举"));
	}
	for attr in ["from", "index", "key", "where-field"] {
		if let Some(path) = node.attr(attr) {
			let segments: Vec<_> = path.strip_prefix('/').unwrap_or(path).split('/').collect();
			if segments.len() != 1
				|| segments
					.iter()
					.any(|s| s.is_empty() || !s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'))
			{
				return Err(node.error("无效数据路径"));
			}
			let valid = if path.starts_with('/') {
				doc.fields.iter().any(|f| f.key == segments[0]) || doc.collections.iter().any(|c| c.name == segments[0])
			} else {
				doc.fields
					.iter()
					.chain(doc.collections.iter().flat_map(|c| &c.fields))
					.any(|f| f.key == segments[0])
			};
			if !valid {
				return Err(node.error("路径引用了未声明的输入"));
			}
		}
	}
	for child in &node.children {
		check_references(child, doc)?;
	}
	Ok(())
}
fn visit(id: &str, doc: &Document, active: &mut BTreeSet<String>, done: &mut BTreeSet<String>) -> Result<(), DslError> {
	if done.contains(id) {
		return Ok(());
	}
	let (kind, name) = id.split_once(':').ok_or_else(|| DslError("无效依赖".into()))?;
	let node = if kind == "condition" {
		doc.conditions.get(name)
	} else {
		doc.values.get(name)
	}
	.ok_or_else(|| DslError("未声明的依赖".into()))?;
	if !active.insert(id.into()) {
		return Err(node.error("定义之间存在循环依赖"));
	}
	let mut refs = BTreeSet::new();
	references(node, &mut refs);
	for dependency in refs {
		visit(&dependency, doc, active, done)?;
	}
	active.remove(id);
	done.insert(id.into());
	Ok(())
}

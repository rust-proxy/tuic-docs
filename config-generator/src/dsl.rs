//! Static XML configuration descriptions, parsed by pest. No callbacks or scripts.
mod parser;

use std::{
	collections::{BTreeMap, BTreeSet},
	fmt,
	net::IpAddr,
};

use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DslError(pub String);
impl fmt::Display for DslError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "配置描述错误：{}", self.0)
	}
}
impl std::error::Error for DslError {}

#[derive(Debug, Clone)]
struct Element {
	tag: String,
	attrs: BTreeMap<String, String>,
	children: Vec<Element>,
	line: usize,
	column: usize,
}
impl Element {
	fn attr(&self, key: &str) -> Option<&str> {
		self.attrs.get(key).map(String::as_str)
	}
	fn error(&self, message: &str) -> DslError {
		DslError(format!("{}:{} <{}> {message}", self.line, self.column, self.tag))
	}
	fn required(&self, key: &str) -> Result<&str, DslError> {
		self.attr(key).ok_or_else(|| self.error(&format!("缺少属性 {key}")))
	}
	fn single(&self) -> Result<&Element, DslError> {
		if self.children.len() != 1 {
			return Err(self.error("需要且只能有一个子元素"));
		}
		Ok(&self.children[0])
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKind {
	Text,
	Password,
	Number,
	Email,
	Toggle,
	Select,
}
#[derive(Debug, Clone)]
pub struct InputField {
	pub key: String,
	pub label: String,
	pub placeholder: String,
	pub hint: String,
	pub section: String,
	pub kind: InputKind,
	pub options: Vec<(String, String)>,
	pub rule: String,
	default: Value,
	when: Option<String>,
}
impl InputField {
	pub fn read_value(&self, row: &Value) -> String {
		row.get(&self.key).map(scalar_text).unwrap_or_default()
	}
	pub fn write_value(&self, row: &mut Value, text: &str) -> bool {
		let value = match &self.default {
			Value::Bool(_) => match text.parse::<bool>() {
				Ok(v) => Value::Bool(v),
				Err(_) => return false,
			},
			Value::Number(_) => match text.parse::<u64>() {
				Ok(v) => v.into(),
				Err(_) => return false,
			},
			_ => Value::String(text.into()),
		};
		if let Some(row) = row.as_object_mut() {
			row.insert(self.key.clone(), value);
			true
		} else {
			false
		}
	}
	pub fn visible_in(&self, doc: &Document, root: &Value, row: &Value) -> Result<bool, DslError> {
		match &self.when {
			Some(name) => doc.condition(name, root, row),
			None => Ok(true),
		}
	}
}
#[derive(Debug, Clone)]
pub struct Collection {
	pub name: String,
	pub fields: Vec<InputField>,
	initial_items: usize,
	when: Option<String>,
}
impl Collection {
	pub fn visible_in(&self, doc: &Document, root: &Value) -> Result<bool, DslError> {
		match &self.when {
			Some(name) => doc.condition(name, root, root),
			None => Ok(true),
		}
	}
}

#[derive(Debug, Clone)]
pub struct Document {
	pub target_version: String,
	pub fields: Vec<InputField>,
	pub collections: Vec<Collection>,
	conditions: BTreeMap<String, Element>,
	values: BTreeMap<String, Element>,
	outputs: Element,
}
impl Document {
	pub fn parse(source: &str) -> Result<Self, DslError> {
		parser::parse(source)
	}
	pub fn defaults(&self) -> Value {
		let mut fields = defaults(&self.fields);
		for collection in &self.collections {
			fields.insert(
				collection.name.clone(),
				Value::Array(vec![Value::Object(defaults(&collection.fields)); collection.initial_items]),
			);
		}
		Value::Object(fields)
	}
	pub fn collection_defaults(&self, name: &str) -> Result<Value, DslError> {
		self.collections
			.iter()
			.find(|c| c.name == name)
			.map(|c| Value::Object(defaults(&c.fields)))
			.ok_or_else(|| DslError("未声明的输入集合".into()))
	}
	pub fn condition(&self, name: &str, root: &Value, row: &Value) -> Result<bool, DslError> {
		let node = self.conditions.get(name).ok_or_else(|| DslError("未声明的条件".into()))?;
		self.test(node, root, row)
	}
	pub fn value(&self, name: &str, root: &Value) -> Result<Value, DslError> {
		let node = self.values.get(name).ok_or_else(|| DslError("未声明的值".into()))?;
		self.eval(node, root, root)
	}
	fn visible(&self, node: &Element, root: &Value, row: &Value) -> Result<bool, DslError> {
		match node.attr("when") {
			Some(name) => self.condition(name, root, row),
			None => Ok(true),
		}
	}
	fn test(&self, node: &Element, root: &Value, row: &Value) -> Result<bool, DslError> {
		match node.tag.as_str() {
			"all" => {
				for c in &node.children {
					if !self.test(c, root, row)? {
						return Ok(false);
					}
				}
				Ok(true)
			}
			"any" => {
				for c in &node.children {
					if self.test(c, root, row)? {
						return Ok(true);
					}
				}
				Ok(false)
			}
			"not" => Ok(!self.test(node.single()?, root, row)?),
			"use" => self.condition(node.required("ref")?, root, row),
			"eq" => {
				let value = self.source(node, root, row)?;
				if !matches!(value, Value::String(_) | Value::Bool(_) | Value::Number(_)) {
					return Err(node.error("比较条件要求标量"));
				}
				Ok(scalar_text(&value) == node.required("value")?)
			}
			"truthy" => self
				.source(node, root, row)?
				.as_bool()
				.ok_or_else(|| node.error("条件要求布尔值")),
			"ip" => Ok(self
				.source(node, root, row)?
				.as_str()
				.ok_or_else(|| node.error("IP 条件要求字符串"))?
				.parse::<IpAddr>()
				.is_ok()),
			_ => Err(node.error("不是条件元素")),
		}
	}
	fn source(&self, node: &Element, root: &Value, row: &Value) -> Result<Value, DslError> {
		if let Some(path) = node.attr("from") {
			return lookup(node, path, root, row).cloned();
		}
		if let Some(name) = node.attr("ref") {
			return self.value(name, root);
		}
		Err(node.error("缺少数据来源"))
	}
	fn eval(&self, node: &Element, root: &Value, row: &Value) -> Result<Value, DslError> {
		if !self.visible(node, root, row)? {
			return Ok(Value::Null);
		}
		let value = match node.tag.as_str() {
			"coalesce" => {
				let mut result = Value::String(String::new());
				for child in &node.children {
					let value = self.eval(child, root, row)?;
					if !value.is_null() && value.as_str() != Some("") {
						result = value;
						break;
					}
				}
				result
			}
			"endpoint" => {
				let host = self.eval(&node.children[0], root, row)?;
				let host = host.as_str().ok_or_else(|| node.error("端点主机必须是字符串"))?;
				let port = self.eval(&node.children[1], root, row)?;
				let port = port
					.as_u64()
					.filter(|p| *p > 0 && *p <= 65535)
					.ok_or_else(|| node.error("端点端口无效"))?;
				if host.parse::<IpAddr>().is_ok_and(|ip| ip.is_ipv6()) {
					format!("[{host}]:{port}").into()
				} else {
					format!("{host}:{port}").into()
				}
			}
			"select" => {
				let items = lookup(node, node.required("from")?, root, row)?
					.as_array()
					.ok_or_else(|| node.error("选择来源必须是列表"))?;
				let index = lookup(node, node.required("index")?, root, row)?
					.as_u64()
					.and_then(|v| usize::try_from(v).ok())
					.ok_or_else(|| node.error("选择索引必须是非负整数"))?;
				let selected = items.get(index).ok_or_else(|| node.error("选择索引超出列表范围"))?;
				self.eval(node.single()?, root, selected)?
			}
			_ => {
				if let Some(value) = node.attr("value") {
					match node.tag.as_str() {
						"boolean" => value
							.parse::<bool>()
							.map(Value::Bool)
							.map_err(|_| node.error("布尔常量无效"))?,
						"integer" => value
							.parse::<i64>()
							.map(Value::from)
							.map_err(|_| node.error("整数常量无效"))?,
						_ => Value::String(value.into()),
					}
				} else if node.children.len() == 1 {
					self.eval(node.single()?, root, row)?
				} else {
					self.source(node, root, row)?
				}
			}
		};
		let value = transform(node, value, node.attr("transform"))?;
		if let Some(unit) = node.attr("unit") {
			if !value.is_number() {
				return Err(node.error("单位要求整数来源"));
			}
			return Ok(format!("{}{unit}", scalar_text(&value)).into());
		}
		Ok(value)
	}
	pub fn project(&self, root: &Value) -> Result<Value, DslError> {
		self.output(&self.outputs, root, root)?
			.ok_or_else(|| self.outputs.error("根输出不可省略"))
	}
	fn output(&self, node: &Element, root: &Value, row: &Value) -> Result<Option<Value>, DslError> {
		if !self.visible(node, root, row)? {
			return Ok(None);
		}
		let result = (|| {
			let value = match node.tag.as_str() {
				"outputs" | "object" => {
					let mut output = Map::new();
					for child in &node.children {
						if let Some(value) = self.output(child, root, row)? {
							output.insert(child.required("name")?.into(), value);
						}
					}
					Value::Object(output)
				}
				"list" | "record" => {
					let literal = [row.clone()];
					let rows = if let Some(path) = node.attr("from") {
						lookup(node, path, root, row)?
							.as_array()
							.ok_or_else(|| node.error("集合来源必须是列表"))?
							.as_slice()
					} else {
						&literal
					};
					let mut items = Vec::new();
					let mut record = Map::new();
					for item in rows {
						if let Some(field) = node.attr("where-field")
							&& scalar_text(lookup(node, field, root, item)?) != node.required("equals")?
						{
							continue;
						}
						if let Some(value) = self.output(node.single()?, root, item)? {
							if node.tag == "record" {
								let key = transform(
									node,
									lookup(node, node.required("key")?, root, item)?.clone(),
									node.attr("key-transform"),
								)?;
								let key = key
									.as_str()
									.filter(|s| !s.is_empty())
									.ok_or_else(|| node.error("映射键必须是非空字符串"))?;
								if record.insert(key.into(), value).is_some() {
									return Err(node.error("映射键在规范化后重复"));
								}
							} else {
								items.push(value);
							}
						}
					}
					if node.tag == "record" {
						Value::Object(record)
					} else {
						Value::Array(items)
					}
				}
				_ => {
					let v = self.eval(node, root, row)?;
					self.scalar_type(node, &v)?;
					v
				}
			};
			if node.attr("omit-empty") == Some("true")
				&& (value.as_array().is_some_and(Vec::is_empty) || value.as_object().is_some_and(Map::is_empty))
			{
				Ok(None)
			} else {
				Ok(Some(value))
			}
		})();
		result.map_err(|e: DslError| DslError(format!("{}.{}", node.attr("name").unwrap_or("[]"), e.0)))
	}
	fn scalar_type(&self, node: &Element, value: &Value) -> Result<(), DslError> {
		let valid = match node.tag.as_str() {
			"string" => value.is_string(),
			"boolean" => value.is_boolean(),
			"integer" => value.as_i64().is_some(),
			"enum" => value.as_str().is_some_and(|v| {
				self.fields
					.iter()
					.any(|f| Some(f.key.as_str()) == node.attr("options") && f.options.iter().any(|(option, _)| option == v))
			}),
			_ => false,
		};
		if valid {
			Ok(())
		} else {
			Err(node.error("输出类型或枚举值不匹配"))
		}
	}
	pub fn redact(&self, value: &Value) -> Result<Value, DslError> {
		self.redact_node(&self.outputs, value)
	}
	fn redact_node(&self, node: &Element, value: &Value) -> Result<Value, DslError> {
		let result = match node.tag.as_str() {
			"outputs" | "object" => {
				let values = value.as_object().ok_or_else(|| node.error("预览对象类型不匹配"))?;
				let mut result = Map::new();
				for (key, value) in values {
					let child = node
						.children
						.iter()
						.find(|n| n.attr("name") == Some(key))
						.ok_or_else(|| node.error("预览含有未声明的字段"))?;
					result.insert(key.clone(), self.redact_node(child, value)?);
				}
				Value::Object(result)
			}
			"list" => value
				.as_array()
				.ok_or_else(|| node.error("预览列表类型不匹配"))?
				.iter()
				.map(|v| self.redact_node(node.single()?, v))
				.collect::<Result<Vec<_>, _>>()?
				.into(),
			"record" => {
				let mut result = Map::new();
				for (k, v) in value.as_object().ok_or_else(|| node.error("预览映射类型不匹配"))? {
					result.insert(k.clone(), self.redact_node(node.single()?, v)?);
				}
				Value::Object(result)
			}
			_ => {
				self.scalar_type(node, value)?;
				value.clone()
			}
		};
		if node.attr("secret") == Some("true") {
			Ok("••••••••".into())
		} else {
			Ok(result)
		}
	}
}
fn defaults(fields: &[InputField]) -> Map<String, Value> {
	fields.iter().map(|f| (f.key.clone(), f.default.clone())).collect()
}
fn scalar_text(value: &Value) -> String {
	match value {
		Value::String(s) => s.clone(),
		Value::Bool(_) | Value::Number(_) => value.to_string(),
		_ => String::new(),
	}
}
fn lookup<'a>(node: &Element, path: &str, root: &'a Value, row: &'a Value) -> Result<&'a Value, DslError> {
	let mut value = if path.starts_with('/') { root } else { row };
	for part in path.trim_start_matches('/').split('/') {
		value = value.get(part).ok_or_else(|| node.error("来源字段不存在"))?;
	}
	Ok(value)
}
fn transform(node: &Element, mut value: Value, transforms: Option<&str>) -> Result<Value, DslError> {
	for op in transforms.unwrap_or_default().split_whitespace() {
		let text = value.as_str().ok_or_else(|| node.error("转换要求字符串来源"))?;
		value = match op {
			"trim" => text.trim().into(),
			"lowercase" => text.to_lowercase().into(),
			"unbracket" => text
				.strip_prefix('[')
				.and_then(|v| v.strip_suffix(']'))
				.unwrap_or(text)
				.into(),
			"integer" => {
				if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
					return Err(node.error("整数转换失败"));
				}
				text.parse::<u64>().map(Value::from).map_err(|_| node.error("整数溢出"))?
			}
			_ => return Err(node.error("未知转换")),
		};
	}
	Ok(value)
}

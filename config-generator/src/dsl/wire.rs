//! Serde wire model. Attributes remain lexical strings until semantic validation:
//! this preserves v3's strict booleans, numeric rules, and empty values.
use std::{collections::BTreeMap, fmt};

use serde::{
	Deserialize, Deserializer,
	de::{self, MapAccess, Visitor},
};

use super::{DslError, Element};

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Node {
	ConfigDsl(Body),
	Inputs(Body),
	Conditions(Body),
	Values(Body),
	Outputs(Body),
	Field(Body),
	Option(Body),
	Collection(Body),
	Condition(Body),
	Value(Body),
	All(Body),
	Any(Body),
	Not(Body),
	Use(Body),
	Eq(Body),
	Truthy(Body),
	Ip(Body),
	Source(Body),
	Coalesce(Body),
	Endpoint(Body),
	Select(Body),
	Object(Body),
	List(Body),
	Record(Body),
	String(Body),
	Boolean(Body),
	Integer(Body),
	Enum(Body),
}

// A streaming Serde visitor keeps recursive stack use independent of the number
// of possible attributes. parser.rs validates permitted attributes per construct.
struct Body {
	attrs: BTreeMap<String, String>,
	children: Vec<Node>,
}

impl<'de> Deserialize<'de> for Body {
	fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		struct BodyVisitor;
		impl<'de> Visitor<'de> for BodyVisitor {
			type Value = Body;

			fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
				formatter.write_str("XML attributes and ordered child elements")
			}

			fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Body, A::Error> {
				let mut attrs = BTreeMap::new();
				let mut children = None;
				while let Some(key) = map.next_key::<String>()? {
					if let Some(name) = key.strip_prefix('@') {
						if attrs.insert(name.to_owned(), map.next_value()?).is_some() {
							return Err(de::Error::custom("duplicate XML attribute"));
						}
					} else if key == "$value" && children.is_none() {
						children = Some(map.next_value()?);
					} else {
						return Err(de::Error::custom("unexpected XML content"));
					}
				}
				Ok(Body {
					attrs,
					children: children.unwrap_or_default(),
				})
			}
		}
		// $value asks quick-xml to deserialize heterogeneous children as one sequence.
		deserializer.deserialize_struct("Body", &["$value"], BodyVisitor)
	}
}

impl Node {
	fn into_element(self, positions: &mut impl Iterator<Item = (usize, usize)>) -> Result<Element, DslError> {
		let (tag, body) = match self {
			Self::ConfigDsl(body) => ("config-dsl", body),
			Self::Inputs(body) => ("inputs", body),
			Self::Conditions(body) => ("conditions", body),
			Self::Values(body) => ("values", body),
			Self::Outputs(body) => ("outputs", body),
			Self::Field(body) => ("field", body),
			Self::Option(body) => ("option", body),
			Self::Collection(body) => ("collection", body),
			Self::Condition(body) => ("condition", body),
			Self::Value(body) => ("value", body),
			Self::All(body) => ("all", body),
			Self::Any(body) => ("any", body),
			Self::Not(body) => ("not", body),
			Self::Use(body) => ("use", body),
			Self::Eq(body) => ("eq", body),
			Self::Truthy(body) => ("truthy", body),
			Self::Ip(body) => ("ip", body),
			Self::Source(body) => ("source", body),
			Self::Coalesce(body) => ("coalesce", body),
			Self::Endpoint(body) => ("endpoint", body),
			Self::Select(body) => ("select", body),
			Self::Object(body) => ("object", body),
			Self::List(body) => ("list", body),
			Self::Record(body) => ("record", body),
			Self::String(body) => ("string", body),
			Self::Boolean(body) => ("boolean", body),
			Self::Integer(body) => ("integer", body),
			Self::Enum(body) => ("enum", body),
		};
		let (line, column) = positions.next().ok_or_else(|| DslError("缺少 XML 元素位置".into()))?;
		let attrs = body.attrs;
		let children = body
			.children
			.into_iter()
			.map(|node| node.into_element(positions))
			.collect::<Result<_, _>>()?;
		Ok(Element {
			tag: tag.into(),
			attrs,
			children,
			line,
			column,
		})
	}
}

pub(super) fn deserialize(source: &str, positions: Vec<(usize, usize)>) -> Result<Element, DslError> {
	let mut deserializer = quick_xml::de::Deserializer::from_str(source);
	let node = Node::deserialize(&mut deserializer).map_err(|_| {
		// Serde errors may contain attribute values. Only expose a source position.
		let offset = deserializer.get_ref().get_ref().buffer_position() as usize;
		let prefix = &source[..offset.min(source.len())];
		let line = prefix.bytes().filter(|b| *b == b'\n').count() + 1;
		let column = prefix.rsplit('\n').next().map_or(1, |s| s.chars().count() + 1);
		DslError(format!("{line}:{column} XML 描述反序列化失败（未知标签、属性或无效结构）"))
	})?;
	node.into_element(&mut positions.into_iter())
}

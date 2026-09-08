use super::{
	parser::{attrs, condition, expression},
	*,
};

#[derive(Debug, Clone)]
pub struct Ui {
	pub title: String,
	pub brand: String,
	pub mark: String,
	pub eyebrow: String,
	pub description: String,
	pub reference: String,
	pub export_hint: String,
	pub mode_field: Option<String>,
	pub format_field: Option<String>,
	pub sections: Vec<Section>,
	pub notices: Vec<Notice>,
}
impl Default for Ui {
	fn default() -> Self {
		Self {
			title: "配置文件生成器".into(),
			brand: "配置工具".into(),
			mark: "C".into(),
			eyebrow: String::new(),
			description: String::new(),
			reference: String::new(),
			export_hint: String::new(),
			mode_field: None,
			format_field: None,
			sections: Vec::new(),
			notices: Vec::new(),
		}
	}
}
#[derive(Debug, Clone)]
pub struct Section {
	pub name: String,
	pub label: String,
	pub detail: String,
	pub collapsed: bool,
	pub when: Option<String>,
	pub notices: Vec<Notice>,
}
#[derive(Debug, Clone)]
pub struct Notice {
	pub text: String,
	pub when: Option<String>,
}
#[derive(Debug, Clone)]
pub struct Export {
	pub name: String,
	pub label: String,
	pub filename: String,
	pub command: String,
	pub when: Option<String>,
}
impl Export {
	pub fn filename(&self, format: &str) -> String {
		format!("{}.{format}", self.filename)
	}
	pub fn command(&self, format: &str) -> String {
		self.command.replace("{filename}", &self.filename(format))
	}
}
#[derive(Debug, Clone)]
pub enum Generator {
	UuidV4,
	Hex(usize),
}
impl Generator {
	pub fn byte_count(&self) -> usize {
		match self {
			Self::UuidV4 => 16,
			Self::Hex(n) => *n,
		}
	}
	pub fn encode(&self, bytes: &[u8]) -> Result<String, DslError> {
		if bytes.len() != self.byte_count() {
			return Err(DslError("随机字节长度错误".into()));
		}
		Ok(match self {
			Self::UuidV4 => {
				let mut data = [0; 16];
				data.copy_from_slice(bytes);
				data[6] = (data[6] & 0x0f) | 0x40;
				data[8] = (data[8] & 0x3f) | 0x80;
				uuid::Uuid::from_bytes(data).hyphenated().to_string()
			}
			Self::Hex(_) => bytes.iter().map(|b| format!("{b:02x}")).collect(),
		})
	}
}
pub(super) fn bounded(node: &Element, key: &str, default: usize, max: usize) -> Result<usize, DslError> {
	node.attr(key).map_or(Ok(default), |s| {
		s.parse().ok().filter(|n| *n <= max).ok_or_else(|| node.error("数值属性越界"))
	})
}
pub(super) fn generator(node: &Element) -> Result<Option<Generator>, DslError> {
	if node.attr("bytes").is_some() && node.attr("generator") != Some("hex") {
		return Err(node.error("bytes 只用于 hex"));
	}
	if node.attr("generator").is_some() && node.attr("type") != Some("string") {
		return Err(node.error("随机生成要求字符串输入"));
	}
	match node.attr("generator") {
		None => Ok(None),
		Some("uuid-v4") => Ok(Some(Generator::UuidV4)),
		Some("hex") => {
			let n = bounded(node, "bytes", 24, 1024)?;
			if n == 0 {
				return Err(node.error("随机字节数必须大于零"));
			}
			Ok(Some(Generator::Hex(n)))
		}
		_ => Err(node.error("未知随机生成方式")),
	}
}
fn notice(node: &Element) -> Result<Notice, DslError> {
	attrs(node, &["text", "when"])?;
	if node.tag != "notice" || !node.children.is_empty() {
		return Err(node.error("需要 notice 元素"));
	}
	Ok(Notice {
		text: node.required("text")?.into(),
		when: node.attr("when").map(str::to_owned),
	})
}
pub(super) fn parse_ui(node: Option<&Element>) -> Result<Ui, DslError> {
	let Some(node) = node else {
		return Ok(Ui::default());
	};
	attrs(
		node,
		&[
			"title",
			"brand",
			"mark",
			"eyebrow",
			"description",
			"reference",
			"export-hint",
			"mode-field",
			"format-field",
		],
	)?;
	let mut ui = Ui {
		title: node.required("title")?.into(),
		brand: node.required("brand")?.into(),
		mark: node.attr("mark").unwrap_or_default().into(),
		eyebrow: node.attr("eyebrow").unwrap_or_default().into(),
		description: node.attr("description").unwrap_or_default().into(),
		reference: node.attr("reference").unwrap_or_default().into(),
		export_hint: node.attr("export-hint").unwrap_or_default().into(),
		mode_field: node.attr("mode-field").map(str::to_owned),
		format_field: node.attr("format-field").map(str::to_owned),
		..Ui::default()
	};
	if !ui.reference.is_empty() && !ui.reference.starts_with("https://") {
		return Err(node.error("说明链接必须使用 HTTPS"));
	}
	let mut seen = BTreeSet::new();
	for child in &node.children {
		if child.tag == "notice" {
			ui.notices.push(notice(child)?);
			continue;
		}
		if child.tag != "section" {
			return Err(child.error("未知界面元素"));
		}
		attrs(child, &["name", "label", "detail", "collapsed", "when"])?;
		let name = child.required("name")?;
		if !seen.insert(name) {
			return Err(child.error("重复界面分区"));
		}
		let collapsed = child
			.attr("collapsed")
			.unwrap_or("false")
			.parse()
			.map_err(|_| child.error("collapsed 必须为布尔值"))?;
		ui.sections.push(Section {
			name: name.into(),
			label: child.required("label")?.into(),
			detail: child.attr("detail").unwrap_or_default().into(),
			collapsed,
			when: child.attr("when").map(str::to_owned),
			notices: child.children.iter().map(notice).collect::<Result<_, _>>()?,
		});
	}
	Ok(ui)
}
pub(super) fn exports(node: &Element) -> Result<Vec<Export>, DslError> {
	node.children
		.iter()
		.map(|n| {
			let name = n.required("name")?;
			let filename = n.attr("filename").unwrap_or(name);
			if filename.is_empty() || !filename.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_') {
				return Err(n.error("文件名必须是静态标识符"));
			}
			Ok(Export {
				name: name.into(),
				label: n.attr("label").unwrap_or(name).into(),
				filename: filename.into(),
				command: n.attr("command").unwrap_or_default().into(),
				when: n.attr("when").map(str::to_owned),
			})
		})
		.collect()
}
pub(super) fn validators(node: Option<&Element>) -> Result<BTreeMap<String, Element>, DslError> {
	let mut result = BTreeMap::new();
	for n in node.into_iter().flat_map(|n| &n.children) {
		attrs(n, &["name", "kind", "message", "min", "max", "transform", "nonblank"])?;
		if n.tag != "validator" || !n.children.is_empty() {
			return Err(n.error("需要 validator 元素"));
		}
		if ![
			"required",
			"length",
			"integer",
			"host",
			"socket",
			"endpoint",
			"email",
			"uuid",
			"domain",
			"public-domain",
			"loopback-socket",
		]
		.contains(&n.required("kind")?)
		{
			return Err(n.error("未知校验类型"));
		}
		n.required("message")?;
		if let Some(v) = n.attr("nonblank")
			&& v.parse::<bool>().is_err()
		{
			return Err(n.error("nonblank 必须为布尔值"));
		}
		for key in ["min", "max"] {
			if let Some(v) = n.attr(key)
				&& v.parse::<u64>().is_err()
			{
				return Err(n.error("校验范围必须为非负整数"));
			}
		}
		if n.attr("min")
			.and_then(|s| s.parse::<u64>().ok())
			.zip(n.attr("max").and_then(|s| s.parse::<u64>().ok()))
			.is_some_and(|(min, max)| min > max)
		{
			return Err(n.error("校验范围颠倒"));
		}
		if result.insert(n.required("name")?.into(), n.clone()).is_some() {
			return Err(n.error("重复校验规则"));
		}
	}
	Ok(result)
}
pub(super) fn rules(node: Option<&Element>) -> Result<Vec<Element>, DslError> {
	let mut result = Vec::new();
	for n in node.into_iter().flat_map(|n| &n.children) {
		attrs(n, &["key", "message", "when", "collection"])?;
		n.required("key")?;
		n.required("message")?;
		match n.tag.as_str() {
			"assert" => condition(n.single()?)?,
			"unique" => {
				n.required("collection")?;
				if n.children.is_empty() {
					return Err(n.error("唯一约束需要键"));
				}
				for c in &n.children {
					expression(c)?;
				}
			}
			_ => return Err(n.error("未知校验声明")),
		}
		result.push(n.clone());
	}
	Ok(result)
}
pub(super) fn resets(node: Option<&Element>) -> Result<Vec<(String, String)>, DslError> {
	node.into_iter()
		.flat_map(|n| &n.children)
		.map(|n| {
			attrs(n, &["on", "target"])?;
			if n.tag != "reset" || !n.children.is_empty() {
				return Err(n.error("需要 reset 元素"));
			}
			Ok((n.required("on")?.into(), n.required("target")?.into()))
		})
		.collect()
}
pub(super) fn check(doc: &mut Document) -> Result<(), DslError> {
	let error = || DslError("界面或校验绑定引用了不存在或类型不符的输入".into());
	for name in [&doc.ui.mode_field, &doc.ui.format_field].into_iter().flatten() {
		let f = doc
			.fields
			.iter()
			.find(|f| &f.key == name && f.kind == InputKind::Select)
			.ok_or_else(error)?;
		if doc.ui.format_field.as_ref() == Some(name)
			&& f.options.iter().any(|(s, _)| !["json", "toml", "yaml"].contains(&s.as_str()))
		{
			return Err(error());
		}
	}
	let mut selections = BTreeSet::new();
	for c in &doc.collections {
		if c.min_items > c.initial_items {
			return Err(error());
		}
		if let Some(name) = &c.selected_by {
			if !selections.insert(name) {
				return Err(error());
			}
			if !doc.fields.iter().any(|f| &f.key == name && f.default.as_u64().is_some()) {
				return Err(error());
			}
		} else if c.all_when.is_some() || c.select_when.is_some() {
			return Err(error());
		}
	}
	if doc.ui.sections.is_empty() {
		let mut sections = BTreeSet::new();
		for name in doc
			.fields
			.iter()
			.map(|f| &f.section)
			.chain(doc.collections.iter().map(|c| &c.section))
		{
			if sections.insert(name.clone()) {
				doc.ui.sections.push(Section {
					name: name.clone(),
					label: if name.is_empty() { "输入".into() } else { name.clone() },
					detail: String::new(),
					collapsed: false,
					when: None,
					notices: Vec::new(),
				});
			}
		}
	} else {
		for f in &doc.fields {
			if doc.ui.mode_field.as_ref() != Some(&f.key)
				&& doc.ui.format_field.as_ref() != Some(&f.key)
				&& !selections.contains(&f.key)
				&& !doc.ui.sections.iter().any(|s| s.name == f.section)
			{
				return Err(error());
			}
		}
		if doc
			.collections
			.iter()
			.any(|c| !doc.ui.sections.iter().any(|s| s.name == c.section))
		{
			return Err(error());
		}
	}
	for n in &doc.rules {
		let fields = if let Some(name) = n.attr("collection") {
			&doc.collections.iter().find(|c| c.name == name).ok_or_else(error)?.fields
		} else {
			&doc.fields
		};
		if !fields.iter().any(|f| Some(f.key.as_str()) == n.attr("key"))
			&& !(n.attr("collection").is_none() && doc.collections.iter().any(|c| Some(c.name.as_str()) == n.attr("key")))
		{
			return Err(error());
		}
	}
	for (on, target) in &doc.resets {
		if !doc.fields.iter().any(|f| &f.key == on) || !doc.fields.iter().any(|f| &f.key == target) {
			return Err(error());
		}
	}
	Ok(())
}

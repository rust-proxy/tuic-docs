//! Check the v3 XML subset before recursive Serde deserialization.
//! This pass records locations and bounds nesting, but does not build the AST.
use quick_xml::{Reader, events::Event};

use super::DslError;

pub(super) fn inspect(source: &str) -> Result<Vec<(usize, usize)>, DslError> {
	if source.len() > 1_048_576 {
		return Err(DslError("描述文件超过 1 MiB".into()));
	}
	let mut lines = vec![0];
	lines.extend(source.match_indices('\n').map(|(offset, _)| offset + 1));
	let location = |offset: usize| {
		let line = lines.partition_point(|start| *start <= offset);
		(line, source[lines[line - 1]..offset].chars().count() + 1)
	};
	let error = |offset, message| {
		let (line, column) = location(offset);
		DslError(format!("{line}:{column} {message}"))
	};
	if let Some((offset, _)) = source.char_indices().find(|(_, c)| !xml_char(*c)) {
		return Err(error(offset, "描述包含无效 XML 字符"));
	}
	if source.starts_with('\u{feff}') {
		return Err(error(0, "不支持 XML BOM"));
	}
	let mut reader = Reader::from_str(source);
	reader.config_mut().check_comments = true;
	let mut depth: usize = 0;
	let mut roots = 0;
	let mut positions = Vec::new();
	loop {
		let offset = reader.buffer_position() as usize;
		let event = reader.read_event().map_err(|_| error(offset, "XML 语法错误"))?;
		match event {
			Event::Start(ref node) | Event::Empty(ref node) => {
				if depth > 64 {
					return Err(error(offset, "元素嵌套超过 64 层"));
				}
				if depth == 0 {
					roots += 1;
					if roots > 1 {
						return Err(error(offset, "只允许一个根元素"));
					}
				}
				if !name(node.name().as_ref()) {
					return Err(error(offset, "无效标签名或不支持命名空间"));
				}
				if !attribute_separators(node.as_ref()) {
					return Err(error(offset, "XML 属性之间必须有空白"));
				}
				for attr in node.attributes() {
					let attr = attr.map_err(|_| error(offset, "XML 属性无效或重复"))?;
					if !name(attr.key.as_ref()) || attr.key.as_ref() == b"xmlns" {
						return Err(error(offset, "无效属性名或不支持命名空间"));
					}
					if attr.value.contains(&b'<') {
						return Err(error(offset, "属性中不能出现未转义的 <"));
					}
					let value = attr
						.decode_and_unescape_value(reader.decoder())
						.map_err(|_| error(offset, "不支持的实体或字符引用"))?;
					if !value.chars().all(xml_char) {
						return Err(error(offset, "属性包含无效 XML 字符"));
					}
				}
				positions.push(location(offset));
				if matches!(event, Event::Start(_)) {
					depth += 1;
				}
			}
			Event::End(_) => {
				depth = depth.checked_sub(1).ok_or_else(|| error(offset, "多余的结束标签"))?;
			}
			Event::Text(text) if text.iter().all(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n')) => {}
			Event::Comment(_) => {}
			Event::Eof if depth == 0 && roots == 1 => return Ok(positions),
			Event::Eof => return Err(error(offset, "XML 不完整或缺少根元素")),
			_ => return Err(error(offset, "不支持 XML 声明、DTD、处理指令、CDATA 或文本")),
		}
	}
}

fn name(bytes: &[u8]) -> bool {
	bytes.first().is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_')
		&& bytes.iter().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}

fn xml_char(c: char) -> bool {
	matches!(c as u32, 0x9 | 0xa | 0xd | 0x20..=0xd7ff | 0xe000..=0xfffd | 0x10000..=0x10ffff)
}

// quick-xml accepts adjacent quoted attributes; v3 requires XML whitespace.
// BytesStart excludes the closing > or />, so a final quote needs no separator.
fn attribute_separators(bytes: &[u8]) -> bool {
	let mut quote = None;
	for (index, byte) in bytes.iter().copied().enumerate() {
		if quote == Some(byte) {
			if bytes
				.get(index + 1)
				.is_some_and(|b| !matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
			{
				return false;
			}
			quote = None;
		} else if quote.is_none() && matches!(byte, b'\'' | b'"') {
			quote = Some(byte);
		}
	}
	true
}

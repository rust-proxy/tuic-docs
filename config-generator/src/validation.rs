use std::{
	collections::{BTreeMap, HashSet},
	net::{IpAddr, Ipv6Addr},
};

use crate::schema::{InputField, InputKind, State, has_client, has_server, input_fields};

pub type Errors = BTreeMap<String, String>;
pub fn host_value(value: &str) -> &str {
	value
		.trim()
		.strip_prefix('[')
		.and_then(|v| v.strip_suffix(']'))
		.unwrap_or(value.trim())
}
pub fn is_domain(value: &str) -> bool {
	!value.is_empty()
		&& value.len() <= 253
		&& !value.bytes().all(|b| b.is_ascii_digit() || b == b'.')
		&& value.split('.').all(|label| {
			!label.is_empty()
				&& label.len() <= 63
				&& label.starts_with(|c: char| c.is_ascii_alphanumeric())
				&& label.ends_with(|c: char| c.is_ascii_alphanumeric())
				&& label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
		})
}
pub fn valid_port(v: &str) -> bool {
	!v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()) && v.parse::<u16>().is_ok_and(|n| n > 0)
}
pub fn positive(v: &str) -> Option<u64> {
	if v.is_empty() || !v.bytes().all(|b| b.is_ascii_digit()) {
		return None;
	}
	v.parse::<u64>().ok().filter(|n| *n > 0 && *n <= 9_007_199_254_740_991)
}
pub fn endpoint(value: &str, literal: bool) -> Option<(String, u16)> {
	let value = value.trim();
	let (host, port) = if let Some(value) = value.strip_prefix('[') {
		let (host, port) = value.split_once("]:")?;
		host.parse::<Ipv6Addr>().ok()?;
		(host, port)
	} else {
		let (host, port) = value.split_once(':')?;
		if host.parse::<IpAddr>().is_err() && (literal || !is_domain(host)) {
			return None;
		}
		(host, port)
	};
	if !valid_port(port) {
		return None;
	}
	Some((host.to_owned(), port.parse().ok()?))
}
pub fn required(v: &str) -> Option<&'static str> {
	v.trim().is_empty().then_some("请填写此项。")
}
pub fn host_rule(v: &str) -> Option<&'static str> {
	let h = host_value(v);
	let ip = h.parse::<IpAddr>();
	((ip.is_err() && !is_domain(h)) || ip.is_ok_and(|ip| ip.is_unspecified()))
		.then_some("请输入可连接的域名或 IP，不含端口、路径或通配监听地址。")
}
pub fn port_rule(v: &str) -> Option<&'static str> {
	(!valid_port(v)).then_some("端口必须是 1–65535 的整数。")
}
pub fn listen_rule(v: &str) -> Option<&'static str> {
	endpoint(v, true)
		.is_none()
		.then_some("请输入有效的 IP:端口；IPv6 使用方括号。")
}
pub fn email_rule(v: &str) -> Option<&'static str> {
	let valid = v.split_once('@').is_some_and(|(local, domain)| {
		!local.is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.') && !domain.contains('@')
	}) && !v.chars().any(char::is_whitespace);
	(!valid).then_some("请填写有效的邮箱地址。")
}
pub fn socks_credential(v: &str) -> Option<&'static str> {
	if v.len() > 255 {
		Some("SOCKS5 认证字段最多 255 字节。")
	} else {
		required(v)
	}
}
pub fn milliseconds(v: &str) -> Option<&'static str> {
	positive(v).is_none().then_some("请输入大于 0 的安全整数（毫秒）。")
}

pub fn check_rule(rule: &str, value: &str) -> Option<&'static str> {
	match rule {
		"" => None,
		"required" => required(value),
		"host" => host_rule(value),
		"port" => port_rule(value),
		"socket" => listen_rule(value),
		"email" => email_rule(value),
		"socks-credential" => socks_credential(value),
		"milliseconds" => milliseconds(value),
		"seconds" => positive(value).is_none().then_some("请输入大于 0 的安全整数（秒）。"),
		"endpoint" => endpoint(value, false).is_none().then_some("目标必须是域名或 IP 加端口。"),
		"password" => value.is_empty().then_some("请输入密码，或点击生成凭据。"),
		"uuid" => {
			let id = value.trim().to_lowercase();
			(!uuid::Uuid::parse_str(&id).is_ok_and(|v| !v.is_nil() && v.hyphenated().to_string() == id))
				.then_some("请输入非全零的标准 UUID，或点击生成凭据。")
		}
		_ => Some("配置描述包含未知校验规则。"),
	}
}
fn check_fields(fields: &[InputField], root: &serde_json::Value, row: &serde_json::Value, prefix: &str, errors: &mut Errors) {
	let Ok(doc) = crate::schema::document() else {
		return;
	};
	for field in fields {
		match field.visible_in(doc, root, row) {
			Ok(false) => continue,
			Err(error) => {
				errors.insert(format!("{prefix}{}", field.key), error.to_string());
				continue;
			}
			Ok(true) => {}
		}
		let value = field.read_value(row);
		let message = if field.kind == InputKind::Select && !field.options.iter().any(|(key, _)| *key == value) {
			Some("请选择有效选项。")
		} else {
			check_rule(&field.rule, &value)
		};
		if let Some(message) = message {
			errors.insert(format!("{prefix}{}", field.key), message.into());
		}
	}
}
pub fn validate(s: &State) -> Errors {
	let mut errors = Errors::new();
	if let Err(error) = crate::schema::document() {
		errors.insert("schema".into(), error.to_string());
		return errors;
	}
	let root = s.value();
	check_fields(input_fields(), &root, &root, "", &mut errors);
	if has_client(s) {
		if !is_domain(&client_sni(s)) {
			errors.insert("sni".into(), "请填写证书对应的 DNS 域名作为 SNI。".into());
		}
		if s.users.get(s.active_user).is_none() {
			errors.insert("activeUser".into(), "请选择一个用户。".into());
		}
		if s.reconnect
			&& let (Some(initial), Some(max)) = (positive(&s.initial_backoff), positive(&s.max_backoff))
			&& max < initial
		{
			errors.insert("maxBackoff".into(), "最大等待不能小于首次等待。".into());
		}
		let mut bindings = HashSet::new();
		for (i, f) in s
			.forwards
			.iter()
			.enumerate()
			.filter(|_| crate::schema::collection_visible("forwards", s))
		{
			let key = format!("forwards.{i}");
			check_fields(
				crate::schema::collection_fields("forwards"),
				&root,
				&root["forwards"][i],
				&format!("{key}."),
				&mut errors,
			);
			if let Some((host, port)) = endpoint(&f.listen, true) {
				let host = host.parse::<IpAddr>().map(|ip| ip.to_string()).unwrap_or(host);
				if !bindings.insert((f.protocol.clone(), host, port)) {
					errors.insert(format!("{key}.listen"), "同一协议的监听地址重复。".into());
				}
				if endpoint(&s.local, true).is_some_and(|(_, socks_port)| socks_port == port) {
					errors.insert(format!("{key}.listen"), "请使用与 SOCKS5 不同的监听端口。".into());
				}
			}
		}
	}
	if has_server(s) {
		let name = server_hostname(s);
		if !is_domain(&name) {
			errors.insert("hostname".into(), "请填写有效的证书域名。".into());
		}
		if s.tls_mode == "acme" && !name.contains('.') {
			errors.insert("hostname".into(), "ACME 需要公开域名。".into());
		}
		if s.tls_mode == "self" && has_client(s) && !s.insecure {
			errors.insert(
				"insecure".into(),
				"自签名测试需要明确开启跳过证书校验，或改用受信任证书。".into(),
			);
		}
	}
	if s.users.is_empty() {
		errors.insert("users".into(), "至少需要一个用户。".into());
	}
	let mut seen = HashSet::new();
	for (i, user) in s.users.iter().enumerate() {
		if !has_server(s) && i != s.active_user {
			continue;
		}
		check_fields(
			crate::schema::collection_fields("users"),
			&root,
			&root["users"][i],
			&format!("users.{i}."),
			&mut errors,
		);
		let id = user.uuid.trim().to_lowercase();
		if !errors.contains_key(&format!("users.{i}.uuid")) && !seen.insert(id) {
			errors.insert(format!("users.{i}.uuid"), "UUID 重复，请为每个用户使用不同 UUID。".into());
		}
	}
	errors
}
fn derived(name: &str, s: &State) -> String {
	crate::schema::document()
		.and_then(|d| d.value(name, &s.value()))
		.ok()
		.and_then(|v| v.as_str().map(str::to_owned))
		.unwrap_or_default()
}
pub fn server_hostname(s: &State) -> String {
	derived("serverHostname", s)
}
pub fn client_sni(s: &State) -> String {
	derived("clientSni", s)
}

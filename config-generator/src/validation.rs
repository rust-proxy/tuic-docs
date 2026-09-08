use std::{
	collections::BTreeMap,
	net::{IpAddr, Ipv6Addr},
};

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

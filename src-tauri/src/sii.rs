use crate::Result;
use regex::Regex;
use std::{collections::HashMap, ops::Range};

#[derive(Debug, Clone)]
pub struct Field {
    pub key: String,
    pub value: String,
    pub span: Range<usize>,
}
#[derive(Debug, Clone)]
pub struct Unit {
    pub kind: String,
    pub id: String,
    pub span: Range<usize>,
    pub body: Range<usize>,
    pub fields: Vec<Field>,
}
impl Unit {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|f| f.key == key)
            .map(|f| f.value.as_str())
    }
    pub fn field(&self, key: &str) -> Result<&Field> {
        self.fields
            .iter()
            .find(|f| f.key == key)
            .ok_or_else(|| format!("{} 缺少字段 {key}", self.id))
    }
    pub fn array(&self, key: &str) -> Result<Vec<String>> {
        let count: usize = self
            .get(key)
            .unwrap_or("0")
            .parse()
            .map_err(|_| format!("无效数组长度 {key}"))?;
        if count > 100_000 {
            return Err("数组过大".into());
        }
        let prefix = format!("{key}[");
        let indexed: Vec<_> = self
            .fields
            .iter()
            .filter(|f| f.key.starts_with(&prefix))
            .collect();
        if indexed.len() != count {
            return Err(format!("{} 的 {key} 数量与引用不一致", self.id));
        }
        let mut out = Vec::with_capacity(count);
        for i in 0..count {
            out.push(self.field(&format!("{key}[{i}]"))?.value.clone());
        }
        Ok(out)
    }
}
#[derive(Debug, Clone)]
pub struct Document {
    pub text: String,
    pub units: Vec<Unit>,
    pub index: HashMap<String, usize>,
    pub closing: usize,
}
#[derive(Debug)]
struct Token {
    v: String,
    s: usize,
    e: usize,
}
fn tokens(text: &str) -> Result<Vec<Token>> {
    let b = text.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < b.len() {
        if b[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if b[i] == b'#' || (b[i] == b'/' && b.get(i + 1) == Some(&b'/')) {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        let s = i;
        if b[i] == b'"' {
            i += 1;
            let mut closed = false;
            while i < b.len() {
                if b[i] == b'\\' {
                    i += 2;
                    continue;
                }
                if b[i] == b'"' {
                    i += 1;
                    closed = true;
                    break;
                }
                i += 1;
            }
            if !closed {
                return Err("字符串未闭合".into());
            }
        } else if b"{}:".contains(&b[i]) {
            i += 1;
        } else {
            while i < b.len() && !b[i].is_ascii_whitespace() && !b"{}:\"".contains(&b[i]) {
                i += 1;
            }
        }
        if i > b.len() {
            return Err("字符串转义不完整".into());
        }
        out.push(Token {
            v: text[s..i].into(),
            s,
            e: i,
        });
    }
    Ok(out)
}
pub fn unquote(s: &str) -> String {
    let s = s.trim();
    let Some(inner) = s.strip_prefix('"').and_then(|s| s.strip_suffix('"')) else {
        return s.into();
    };
    let bytes = inner.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            if bytes[i + 1] == b'x' && i + 3 < bytes.len() {
                if let (Some(a), Some(b)) = (
                    (bytes[i + 2] as char).to_digit(16),
                    (bytes[i + 3] as char).to_digit(16),
                ) {
                    out.push((a * 16 + b) as u8);
                    i += 4;
                    continue;
                }
            }
            if matches!(bytes[i + 1], b'\\' | b'"') {
                out.push(bytes[i + 1]);
                i += 2;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| inner.into())
}
pub fn quoted(s: &str) -> Result<String> {
    if s.contains('\0') {
        return Err("文本含有不允许的字符".into());
    }
    let mut out = String::from("\"");
    for b in s.bytes() {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\\""),
            32..=126 => out.push(b as char),
            _ => out.push_str(&format!("\\x{b:02x}")),
        }
    }
    out.push('"');
    Ok(out)
}
impl Document {
    pub fn parse(text: String) -> Result<Self> {
        let tok = tokens(&text)?;
        if tok.len() < 3 || tok[0].v != "SiiNunit" || tok[1].v != "{" {
            return Err("不是明文 SII 存档".into());
        }
        let field_re =
            Regex::new(r"(?m)^[ \t]*([A-Za-z_][A-Za-z0-9_]*(?:\[[0-9]*\])?):[ \t]*([^\r\n]*)")
                .unwrap();
        let mut i = 2;
        let mut units = Vec::new();
        let mut index = HashMap::new();
        while i < tok.len() && tok[i].v != "}" {
            if i + 3 >= tok.len() || tok[i + 1].v != ":" || tok[i + 3].v != "{" {
                return Err(format!("无法识别 unit，字节位置 {}", tok[i].s));
            }
            let header = i;
            let body_start = tok[i + 3].e;
            i += 4;
            let mut depth = 1;
            while i < tok.len() {
                if tok[i].v == "{" {
                    depth += 1;
                }
                if tok[i].v == "}" {
                    depth -= 1;
                }
                if depth == 0 {
                    break;
                }
                i += 1;
            }
            if i >= tok.len() {
                return Err("unit 未闭合".into());
            }
            let body = body_start..tok[i].s;
            let mut fields = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for cap in field_re.captures_iter(&text[body.clone()]) {
                let k = cap[1].to_string();
                let val = cap.get(2).unwrap();
                // Save arrays are indexed. Repeated [] is permitted in definitions.
                if !k.ends_with("[]") && !seen.insert(k.clone()) {
                    return Err(format!("重复字段 {k}"));
                }
                fields.push(Field {
                    key: k,
                    value: val.as_str().trim_end().into(),
                    span: (body.start + val.start())..(body.start + val.end()),
                });
            }
            let id = tok[header + 2].v.clone();
            if index.insert(id.clone(), units.len()).is_some() {
                return Err(format!("重复 unit ID {id}"));
            }
            units.push(Unit {
                kind: tok[header].v.clone(),
                id,
                span: tok[header].s..tok[i].e,
                body,
                fields,
            });
            i += 1;
        }
        if i != tok.len() - 1 || tok[i].v != "}" {
            return Err("SII 文件外层结构不完整".into());
        }
        Ok(Self {
            text,
            units,
            index,
            closing: tok[i].s,
        })
    }
    pub fn unit(&self, id: &str) -> Result<&Unit> {
        self.index
            .get(id)
            .map(|&n| &self.units[n])
            .ok_or_else(|| format!("找不到引用 {id}"))
    }
    pub fn patch(&self, mut edits: Vec<(Range<usize>, String)>) -> Result<Self> {
        edits.sort_by_key(|e| e.0.start);
        for pair in edits.windows(2) {
            if pair[0].0.end > pair[1].0.start {
                return Err("修改范围重叠".into());
            }
        }
        let mut text = self.text.clone();
        for (span, value) in edits.into_iter().rev() {
            text.replace_range(span, &value);
        }
        Self::parse(text)
    }
    pub fn replace(&self, id: &str, key: &str, value: String) -> Result<Self> {
        let f = self.unit(id)?.field(key)?;
        self.patch(vec![(f.span.clone(), value)])
    }
    pub fn validate_vehicles(&self) -> Result<()> {
        for u in &self.units {
            if u.kind == "vehicle" {
                let refs = u.array("accessories")?;
                let mut seen = std::collections::HashSet::new();
                for r in refs {
                    if !seen.insert(r.clone()) {
                        return Err("重复配件引用".into());
                    }
                    self.unit(&r)?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_unknown_and_escaped_braces() {
        let s="SiiNunit\r\n{\r\nvehicle : _a {\r\n accessories: 1\r\n accessories[0]: _b\r\n weird: &ff123456\r\n}\r\nvehicle_accessory : _b {\r\n data_path: \"/def/{x}.sii\"\r\n unknown: \"a\\\"}b\"\r\n}\r\n}\r\n";
        let d = Document::parse(s.into()).unwrap();
        d.validate_vehicles().unwrap();
        assert_eq!(d.text, s);
        let p = d
            .replace("_b", "data_path", quoted("/def/new.sii").unwrap())
            .unwrap();
        assert_eq!(p.text, s.replace("/def/{x}.sii", "/def/new.sii"));
    }
    #[test]
    fn rejects_duplicate_and_missing() {
        assert!(Document::parse("SiiNunit { a : _a {} a : _a {} }".into()).is_err());
        let d = Document::parse(
            "SiiNunit {\nvehicle : _a {\n accessories: 1\n accessories[0]: _missing\n}\n}".into(),
        )
        .unwrap();
        assert!(d.validate_vehicles().is_err());
    }
}

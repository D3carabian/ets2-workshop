use crate::Result;
use regex::Regex;
use std::{collections::HashMap, ops::Range, sync::OnceLock};

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
    /// Validate fields consumed from a read-only definition before using `get`.
    /// A base name also selects its indexed entries; `[]` entries remain additive.
    pub fn validate_unique_values(&self, keys: &[&str]) -> Result<()> {
        let mut seen = HashMap::new();
        for field in &self.fields {
            let base = field.key.split('[').next().unwrap_or(&field.key);
            if field.key.ends_with("[]")
                || (!keys.contains(&field.key.as_str()) && !keys.contains(&base))
            {
                continue;
            }
            if let Some(previous) = seen.insert(field.key.as_str(), field.value.as_str()) {
                if previous != field.value {
                    return Err(format!("{} 的字段 {} 有多个不同值", self.id, field.key));
                }
            }
        }
        Ok(())
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
        let indexed: HashMap<_, _> = indexed.iter().map(|f| (f.key.as_str(), *f)).collect();
        let mut out = Vec::with_capacity(count);
        for i in 0..count {
            let name = format!("{key}[{i}]");
            let field = indexed
                .get(name.as_str())
                .ok_or_else(|| format!("{} 缺少字段 {name}", self.id))?;
            out.push(field.value.clone());
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
    comments: Vec<Range<usize>>,
}
#[derive(Debug)]
struct Token {
    v: String,
    s: usize,
    e: usize,
}
struct Lexed {
    masked: String,
    tokens: Vec<Token>,
    comments: Vec<Range<usize>>,
    line_ends: Vec<usize>,
}

/// Hide comments without changing byte offsets, line endings, or quoted strings.
pub fn mask_comments(text: &str) -> Result<String> {
    Ok(tokens(text)?.masked)
}

/// Read standalone include directives using the same string/comment boundaries as SII.
/// Ranges cover the directive and quoted path, leaving trailing comments and newlines intact.
pub fn include_directives(text: &str) -> Result<Vec<(Range<usize>, String)>> {
    let lexed = tokens(text)?;
    let mut directives = Vec::new();
    let mut first = 0;
    for end in lexed
        .line_ends
        .iter()
        .copied()
        .chain(std::iter::once(text.len()))
    {
        let mut next = first;
        while next < lexed.tokens.len() && lexed.tokens[next].s < end {
            next += 1;
        }
        let line = &lexed.tokens[first..next];
        first = next;
        let Some(directive) = line.first().filter(|token| token.v == "@include") else {
            continue;
        };
        if line.len() != 2 || !line[1].v.starts_with('"') {
            return Err(format!("include 指令无效，字节位置 {}", directive.s));
        }
        directives.push((directive.s..line[1].e, unquote(&line[1].v)));
    }
    Ok(directives)
}

fn comment_start(b: &[u8], i: usize) -> bool {
    b[i] == b'#' || (b[i] == b'/' && matches!(b.get(i + 1), Some(b'/' | b'*')))
}

fn tokens(text: &str) -> Result<Lexed> {
    let b = text.as_bytes();
    let mut masked = b.to_vec();
    let mut i = 0;
    let mut out = Vec::new();
    let mut comments = Vec::new();
    let mut line_ends = Vec::new();
    while i < b.len() {
        if b[i].is_ascii_whitespace() {
            if matches!(b[i], b'\r' | b'\n') {
                line_ends.push(i);
            }
            i += 1;
            continue;
        }
        if comment_start(b, i) {
            let start = i;
            if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
                i += 2;
                while i + 1 < b.len() && &b[i..i + 2] != b"*/" {
                    i += 1;
                }
                if i + 1 >= b.len() {
                    return Err(format!("块注释未闭合，字节位置 {start}"));
                }
                i += 2;
            } else {
                while i < b.len() && !matches!(b[i], b'\r' | b'\n') {
                    i += 1;
                }
            }
            for byte in &mut masked[start..i] {
                if !matches!(*byte, b'\r' | b'\n') {
                    *byte = b' ';
                }
            }
            comments.push(start..i);
            continue;
        }
        let s = i;
        if b[i] == b'"' {
            i += 1;
            let mut closed = false;
            while i < b.len() {
                if b[i] == b'\\' {
                    if i + 1 >= b.len() {
                        return Err("字符串转义不完整".into());
                    }
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
            while i < b.len()
                && !b[i].is_ascii_whitespace()
                && !b"{}:\"".contains(&b[i])
                && !comment_start(b, i)
            {
                i += 1;
            }
        }
        out.push(Token {
            v: text[s..i].into(),
            s,
            e: i,
        });
    }
    Ok(Lexed {
        // Entire comments are replaced, including every byte of UTF-8 characters.
        masked: String::from_utf8(masked).expect("comment masking preserves UTF-8"),
        tokens: out,
        comments,
        line_ends,
    })
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
        Self::parse_with_duplicates(text, false)
    }
    /// Read definitions without assigning override precedence to repeated fields.
    /// Consumers must validate the fields they use; save parsing and edits stay strict.
    pub fn parse_definition(text: String) -> Result<Self> {
        Self::parse_with_duplicates(text, true)
    }
    fn parse_with_duplicates(text: String, allow_duplicate_fields: bool) -> Result<Self> {
        let lexed = tokens(&text)?;
        let tok = &lexed.tokens;
        if tok.len() < 3 || tok[0].v != "SiiNunit" || tok[1].v != "{" {
            return Err("不是明文 SII 存档".into());
        }
        static FIELD_RE: OnceLock<Regex> = OnceLock::new();
        let field_re = FIELD_RE.get_or_init(|| {
            Regex::new(r"(?s)^\s*([A-Za-z_][A-Za-z0-9_]*(?:\[[0-9]*\])?):(.*)$").unwrap()
        });
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
            // Only newlines outside comments and strings end a field. In particular,
            // a block comment may separate a key from its value across several lines.
            let mut start = body.start;
            let first_end = lexed.line_ends.partition_point(|&end| end < start);
            for end in lexed.line_ends[first_end..]
                .iter()
                .copied()
                .take_while(|&end| end < body.end)
                .chain(std::iter::once(body.end))
            {
                let line_start = start;
                start = end + 1;
                let Some(cap) = field_re.captures(&lexed.masked[line_start..end]) else {
                    continue;
                };
                let k = cap[1].to_string();
                let val = cap.get(2).unwrap();
                let value = val.as_str().trim();
                let value_start = if value.is_empty() {
                    // An empty value with a trailing comment must be filled before
                    // the comment, otherwise a line comment would hide the new value.
                    let raw = &text[line_start + val.start()..end];
                    line_start + val.start() + raw.len() - raw.trim_start_matches([' ', '\t']).len()
                } else {
                    line_start + val.start() + val.as_str().len() - val.as_str().trim_start().len()
                };
                // Save arrays are indexed. Repeated [] is permitted in definitions.
                if !allow_duplicate_fields && !k.ends_with("[]") && !seen.insert(k.clone()) {
                    return Err(format!("重复字段 {k}"));
                }
                fields.push(Field {
                    key: k,
                    value: value.into(),
                    span: value_start..value_start + value.len(),
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
            comments: lexed.comments,
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
        for (span, mut value) in edits.into_iter().rev() {
            if self
                .units
                .iter()
                .flat_map(|unit| &unit.fields)
                .any(|field| field.span == span)
            {
                // A contiguous field span can contain internal block comments.
                // Keep those comments immediately after the replacement value;
                // comments outside the value span remain at their original positions.
                let first = self
                    .comments
                    .partition_point(|comment| comment.start < span.start);
                for comment in self.comments[first..]
                    .iter()
                    .take_while(|comment| comment.end <= span.end)
                {
                    value.push(' ');
                    value.push_str(&self.text[comment.clone()]);
                }
            }
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
    fn definitions_retain_duplicate_fields_but_saves_and_patches_stay_strict() {
        let source = concat!(
            "SiiNunit {\naccessory_interior_data : interior {\n",
            " name: \"Interior\"\n",
            " dashboard_path: \"same.pmd\"\n",
            " dashboard_path: \"same.pmd\" // repeated include\n",
            " gps_path: \"first.pmd\"\n",
            " gps_path: \"second.pmd\"\n}\n}"
        );
        assert!(Document::parse(source.into())
            .unwrap_err()
            .contains("重复字段"));
        let document = Document::parse_definition(source.into()).unwrap();
        let unit = document.unit("interior").unwrap();
        assert_eq!(unit.fields.len(), 5);
        assert_eq!(document.text, source);
        assert_eq!(unit.fields[3].value, "\"first.pmd\"");
        assert_eq!(unit.fields[4].value, "\"second.pmd\"");
        unit.validate_unique_values(&["name", "dashboard_path"])
            .unwrap();
        assert!(unit.validate_unique_values(&["gps_path"]).is_err());
        assert!(document
            .replace("interior", "name", "\"Other\"".into())
            .is_err());
        assert!(document.patch(vec![]).is_err());
        assert!(Document::parse_definition(
            "SiiNunit { a : duplicate {} a : duplicate {} }".into()
        )
        .is_err());
    }

    #[test]
    fn definition_consumers_reject_ambiguous_scalars_and_indices() {
        let document = Document::parse_definition(
            concat!(
                "SiiNunit {\naccessory_engine_data : engine {\n",
                " name: \"same\"\n name: \"same\"\n",
                " torque: 100\n torque: 200\n",
                " suitable_for[0]: \"same\"\n suitable_for[0]: \"same\"\n",
                " suitable_for[]: \"first\"\n suitable_for[]: \"second\"\n",
                " require[0]: \"first\"\n require[0]: \"second\"\n}\n}"
            )
            .into(),
        )
        .unwrap();
        let unit = document.unit("engine").unwrap();
        unit.validate_unique_values(&["name", "suitable_for"])
            .unwrap();
        assert!(unit
            .validate_unique_values(&["torque"])
            .unwrap_err()
            .contains("torque"));
        assert!(unit
            .validate_unique_values(&["require"])
            .unwrap_err()
            .contains("require[0]"));
        assert!(unit.validate_unique_values(&["require[0]"]).is_err());
        unit.validate_unique_values(&["require[1]"]).unwrap();
        assert_eq!(
            unit.fields
                .iter()
                .filter(|f| f.key == "suitable_for[]")
                .count(),
            2
        );
    }

    #[test]
    fn includes_use_logical_lines_and_preserve_trailing_comments() {
        let source = concat!(
            "/*\r\n@include \"commented.sii\"\r\n*/\r\n",
            "text: \"multiline\r\n@include \\\"in-string.sii\\\"\r\nend\"\r\n",
            " // @include \"also-commented.sii\"\r\n",
            " /* leading */ @include /* between\r\npath */ \"中文.sii\" // tail\r\n",
            "@include \"second.sii\"#last\r\n"
        );
        let includes = include_directives(source).unwrap();
        assert_eq!(includes.len(), 2);
        assert_eq!(includes[0].1, "中文.sii");
        assert_eq!(
            &source[includes[0].0.clone()],
            "@include /* between\r\npath */ \"中文.sii\""
        );
        assert!(source[includes[0].0.end..].starts_with(" // tail\r\n"));
        assert_eq!(includes[1].1, "second.sii");
        assert!(source[includes[1].0.end..].starts_with("#last\r\n"));
    }

    #[test]
    fn include_paths_consume_complete_escaped_strings() {
        let source = r#"@include "dir\\with\"quote#///*.sii" /* tail */"#;
        let includes = include_directives(source).unwrap();
        assert_eq!(includes.len(), 1);
        assert_eq!(includes[0].1, "dir\\with\"quote#///*.sii");
        assert_eq!(
            &source[includes[0].0.clone()],
            r#"@include "dir\\with\"quote#///*.sii""#
        );
        for invalid in [
            "@include bare.sii",
            "@include \"first.sii\" \"second.sii\"",
            "@include \"unterminated.sii",
            "@include \"unescaped\"quote.sii\"",
        ] {
            assert!(include_directives(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn masks_comments_without_changing_strings_offsets_or_line_endings() {
        let literal = r#""中文 # // /* \\ \" */""#;
        let source = format!("{literal} # 尾注释\r\n// next\r\n/* 多行\r\n块 */ token");
        let masked = mask_comments(&source).unwrap();
        assert_eq!(masked.len(), source.len());
        assert!(masked.starts_with(literal));
        assert_eq!(masked.find("token"), source.find("token"));
        for (original, replacement) in source.bytes().zip(masked.bytes()) {
            if matches!(original, b'\r' | b'\n') {
                assert_eq!(replacement, original);
            }
        }
        assert!(!masked.contains("尾注释"));
        assert!(!masked.contains("next"));
        assert!(!masked.contains("多行"));
    }

    #[test]
    fn field_values_exclude_trailing_comments_and_patch_preserves_them() {
        let source = concat!(
            "SiiNunit\r\n{\r\nvehicle : _a {\r\n",
            " accessories: 1# 数量\r\n",
            " accessories[0]: _b // reference\r\n",
            " unknown: &ff123456 /* keep */\r\n",
            "}\r\nvehicle_accessory : _b {\r\n",
            " data_path: \"/def/旧.sii\"  // path\r\n",
            "}\r\n}\r\n"
        );
        let document = Document::parse(source.into()).unwrap();
        document.validate_vehicles().unwrap();
        assert_eq!(
            document.unit("_a").unwrap().get("unknown"),
            Some("&ff123456")
        );
        let patched = document
            .replace("_b", "data_path", quoted("/def/new.sii").unwrap())
            .unwrap();
        assert_eq!(patched.text, source.replace("/def/旧.sii", "/def/new.sii"));
        assert_eq!(
            document
                .replace("_a", "accessories", "2".into())
                .unwrap()
                .text,
            source.replace("accessories: 1#", "accessories: 2#")
        );
    }

    #[test]
    fn quoted_comment_markers_and_escaped_quotes_remain_values() {
        let value = r#""中文 # // /* \\ \" */""#;
        let source = format!("SiiNunit {{\na : _a {{\n text: {value} // comment\n}}\n}}");
        let document = Document::parse(source.clone()).unwrap();
        let field = document.unit("_a").unwrap().field("text").unwrap();
        assert_eq!(field.value, value);
        assert_eq!(&source[field.span.clone()], value);
        assert_eq!(unquote(&field.value), "中文 # // /* \\ \" */");
        assert_eq!(document.text, source);
    }

    #[test]
    fn block_comments_hide_fake_units_fields_and_includes() {
        let source = concat!(
            "SiiNunit\n{\n/*\n bogus : _fake {\n",
            " @include \"missing.sii\"\n }\n*/\n",
            "a /* header */ : _a {\n",
            " /*\n fake: 123\n }\n */\n",
            " count: /* before\n value */ 25 /* trailing\n comment */\n",
            " text: \"first\n fake: still a string\"\n",
            " next: valid\n}\n}\n"
        );
        let document = Document::parse(source.into()).unwrap();
        assert_eq!(document.units.len(), 1);
        let unit = document.unit("_a").unwrap();
        assert_eq!(unit.fields.len(), 3);
        assert_eq!(unit.get("count"), Some("25"));
        assert_eq!(unit.get("text"), Some("\"first\n fake: still a string\""));
        assert_eq!(unit.get("next"), Some("valid"));
        assert!(!mask_comments(source).unwrap().contains("@include"));
        assert_eq!(
            document.replace("_a", "count", "30".into()).unwrap().text,
            source.replace("*/ 25 /*", "*/ 30 /*")
        );
    }

    #[test]
    fn patch_keeps_internal_block_comments_and_following_fields() {
        let source = concat!(
            "SiiNunit\r\n{\r\na : _a {\r\n",
            " tuple: (1, /* 中文\r\n note */ 2) // tail\r\n",
            " unknown: unchanged\r\n}\r\n}\r\n"
        );
        let document = Document::parse(source.into()).unwrap();
        let field = document.unit("_a").unwrap().field("tuple").unwrap();
        let patched = document
            .patch(vec![(field.span.clone(), "(3, 4)".into())])
            .unwrap();
        assert_eq!(
            patched.text,
            source.replace("(1, /* 中文\r\n note */ 2)", "(3, 4) /* 中文\r\n note */")
        );
        let unit = patched.unit("_a").unwrap();
        assert_eq!(unit.get("tuple"), Some("(3, 4)"));
        assert_eq!(unit.get("unknown"), Some("unchanged"));
    }

    #[test]
    fn empty_value_replacement_precedes_comment() {
        let source = "SiiNunit {\na : _a {\n value: // retained\n next: 1\n}\n}";
        let document = Document::parse(source.into()).unwrap();
        let patched = document.replace("_a", "value", "2 ".into()).unwrap();
        assert_eq!(patched.text, source.replace("value: //", "value: 2 //"));
        assert_eq!(patched.unit("_a").unwrap().get("value"), Some("2"));
    }

    #[test]
    fn rejects_unclosed_comments_and_strings() {
        for source in ["SiiNunit { /*", "SiiNunit { /* 中文\r\n *"] {
            assert!(mask_comments(source).unwrap_err().contains("块注释未闭合"));
            assert!(Document::parse(source.into()).is_err());
        }
        assert!(mask_comments("\"unclosed").is_err());
        assert!(mask_comments("\"incomplete\\").is_err());
        let document =
            Document::parse("SiiNunit {\na : _a {\n n: 25/* gap */00\n}\n}".into()).unwrap();
        assert!(document
            .unit("_a")
            .unwrap()
            .get("n")
            .unwrap()
            .parse::<u32>()
            .is_err());
    }

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

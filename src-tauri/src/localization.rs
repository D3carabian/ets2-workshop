//! Game-owned localized text stays in the player's local catalog, never in release assets.
use crate::{
    sii::{unquote, Document},
    Result,
};
use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
    sync::OnceLock,
};

pub type Dictionary = BTreeMap<String, String>;
pub type Languages = BTreeMap<String, Dictionary>;

// Independent of accessory parser freshness: older catalogs remain editable.
pub const DRIVER_NAMES_SCHEMA: u32 = 1;

pub fn parse_driver_names(text: &str) -> Result<HashMap<String, String>> {
    let doc = Document::parse(text.trim_start_matches('\u{feff}').to_owned())?;
    let mut names = HashMap::new();
    let mut found = false;
    for unit in doc.units.iter().filter(|unit| unit.kind == "driver_names") {
        found = true;
        for field in &unit.fields {
            let Some(index) = field
                .key
                .strip_prefix("name[")
                .and_then(|s| s.strip_suffix(']'))
            else {
                continue;
            };
            if index.is_empty() || !index.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err("驾驶员姓名缺少明确的数字索引".into());
            }
            let index = index.parse::<u32>().map_err(|_| "驾驶员姓名索引过大")?;
            let name = unquote(&field.value)
                .trim_start_matches('+')
                .trim()
                .to_owned();
            if name.is_empty() || names.len() >= 100_000 {
                return Err("驾驶员姓名为空或数量超限".into());
            }
            if names.insert(format!("driver.{index}"), name).is_some() {
                return Err("驾驶员姓名存在重复索引".into());
            }
        }
    }
    if !found || names.is_empty() {
        return Err("未找到有效的驾驶员姓名表".into());
    }
    Ok(names)
}

/// Read game-owned names only while building the catalog, never per save/truck.
pub(crate) fn load_driver_names(
    game: &Path,
    sources: &[crate::catalog::Source],
) -> (HashMap<String, String>, Vec<String>, bool) {
    let loaded = (|| -> Result<HashMap<String, String>> {
        let locale = game.join("locale.scs");
        let archive = match std::fs::metadata(&locale) {
            Ok(_) => Some(crate::game_archive::Archive::open(&locale)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.to_string()),
        };
        let read = |path: &str| -> Result<Option<String>> {
            for source in sources.iter().rev() {
                if let Some(bytes) = source.read(path, 16 * 1024 * 1024)? {
                    return crate::game_archive::decode_text(&bytes).map(Some);
                }
            }
            archive
                .as_ref()
                .map(|archive| archive.read(path))
                .transpose()?
                .flatten()
                .map(|bytes| crate::game_archive::decode_text(&bytes))
                .transpose()
        };
        match expand(
            "locale/en_gb/driver_names.sii",
            &read,
            &mut Vec::new(),
            &mut 0,
        )? {
            Some(text) => parse_driver_names(&text),
            None => Ok(HashMap::new()),
        }
    })();
    match loaded {
        Ok(names) => (names, Vec::new(), true),
        Err(error) => (
            HashMap::new(),
            vec![format!("驾驶员姓名未读取，将显示编号：{error}")],
            false,
        ),
    }
}

fn array(unit: &crate::sii::Unit, key: &str) -> Result<Vec<String>> {
    let anonymous = format!("{key}[]");
    let prefix = format!("{key}[");
    let fields: Vec<_> = unit
        .fields
        .iter()
        .filter(|field| field.key.starts_with(&prefix))
        .collect();
    if fields.iter().any(|field| field.key == anonymous) {
        if fields.iter().any(|field| field.key != anonymous) {
            return Err("语言文本混用了隐式与显式数组索引".into());
        }
        if let Some(count) = unit.get(key) {
            if count.parse::<usize>().ok() != Some(fields.len()) {
                return Err("语言文本数组长度错误".into());
            }
        }
        return Ok(fields
            .into_iter()
            .map(|field| field.value.clone())
            .collect());
    }
    unit.array(key)
}

pub fn parse(text: &str) -> Result<Dictionary> {
    let doc = Document::parse(text.trim_start_matches('\u{feff}').to_owned())?;
    let mut values = Dictionary::new();
    let mut found = false;
    for unit in doc.units.iter().filter(|u| u.kind == "localization_db") {
        found = true;
        let keys = array(unit, "key")?;
        let translated = array(unit, "val")?;
        if keys.len() != translated.len() {
            return Err("游戏语言文本的键和值数量不一致".into());
        }
        for (key, value) in keys.iter().zip(translated) {
            values.insert(unquote(key), unquote(&value));
        }
    }
    if !found {
        return Err("未找到游戏语言文本".into());
    }
    Ok(values)
}

fn include_path(parent: &str, relative: &str) -> Result<String> {
    if relative.contains(['\\', ':', '\0']) {
        return Err("无效语言文件引用".into());
    }
    let joined = if relative.starts_with('/') {
        relative.trim_start_matches('/').to_owned()
    } else {
        format!(
            "{}/{}",
            parent.rsplit_once('/').map(|(dir, _)| dir).unwrap_or(""),
            relative
        )
    };
    let mut path = Vec::new();
    for part in joined.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if path.pop().is_none() {
                    return Err("语言文件引用越界".into());
                }
            }
            _ => path.push(part),
        }
    }
    let path = path.join("/");
    if !path.starts_with("locale/") {
        return Err("语言文件引用越界".into());
    }
    Ok(path)
}

fn expand(
    path: &str,
    read: &impl Fn(&str) -> Result<Option<String>>,
    stack: &mut Vec<String>,
    budget: &mut usize,
) -> Result<Option<String>> {
    const MAX_TEXT: usize = 16 * 1024 * 1024;
    if stack.len() >= 12 || stack.iter().any(|entry| entry == path) {
        return Err("语言文件循环引用或引用过深".into());
    }
    let Some(text) = read(path)? else {
        return Ok(None);
    };
    *budget = budget.checked_add(text.len()).ok_or("语言文本过大")?;
    if *budget > MAX_TEXT {
        return Err("语言文本过大".into());
    }
    let mut output = String::new();
    let mut end = 0;
    stack.push(path.to_owned());
    for (matched, relative) in crate::sii::include_directives(&text)? {
        output.push_str(&text[end..matched.start]);
        let child = include_path(path, &relative)?;
        output.push_str(
            &expand(&child, read, stack, budget)?.ok_or_else(|| format!("缺少语言引用 {child}"))?,
        );
        end = matched.end;
    }
    output.push_str(&text[end..]);
    stack.pop();
    Ok(Some(output))
}

/// Read only the installed game's en_gb and zh_cn locale trees; no network or save access.
pub fn load(game: &Path, roots: &[(PathBuf, String)]) -> (Languages, Vec<String>) {
    load_sources(game, &crate::catalog::disk_sources(roots))
}

pub(crate) fn load_sources(
    game: &Path,
    sources: &[crate::catalog::Source],
) -> (Languages, Vec<String>) {
    let mut languages = Languages::from([
        ("en".into(), Dictionary::new()),
        ("zh_cn".into(), Dictionary::new()),
    ]);
    let mut warnings = Vec::new();
    let archive = match crate::game_archive::Archive::open(&game.join("locale.scs")) {
        Ok(archive) => Some(archive),
        Err(error) => {
            warnings.push(format!(
                "游戏名称文本 {} 未读取：{error}。未解析名称仍可按原始标识查看。",
                game.join("locale.scs").display()
            ));
            None
        }
    };
    let archive_read = |path: &str| -> Result<Option<String>> {
        archive
            .as_ref()
            .map(|archive| archive.read(path))
            .transpose()?
            .flatten()
            .map(|bytes| crate::game_archive::decode_text(&bytes))
            .transpose()
    };
    for (language, directory) in [("en", "en_gb"), ("zh_cn", "zh_cn")] {
        let read_into = |path: &str,
                         reader: &dyn Fn(&str) -> Result<Option<String>>,
                         languages: &mut Languages,
                         warnings: &mut Vec<String>| match expand(
            path,
            &reader,
            &mut Vec::new(),
            &mut 0,
        )
        .and_then(|text| text.map(|text| parse(&text)).transpose())
        {
            Ok(Some(dictionary)) => languages.get_mut(language).unwrap().extend(dictionary),
            Ok(None) => {}
            Err(error) => warnings.push(format!("语言文件 {path} 未加载：{error}")),
        };
        for filename in ["local.sii", "local.steam.sii", "local.override.sii"] {
            read_into(
                &format!("locale/{directory}/{filename}"),
                &archive_read,
                &mut languages,
                &mut warnings,
            );
        }
        for (index, source) in sources.iter().enumerate() {
            let files = source.paths(&format!("locale/{directory}"), false, &mut warnings);
            for path in files.into_iter().filter(|path| {
                path.rsplit('/')
                    .next()
                    .is_some_and(|name| name.starts_with("local") && name.ends_with(".sii"))
            }) {
                let entry_path = path.as_str();
                let local_read = |path: &str| -> Result<Option<String>> {
                    match source.read(path, 16 * 1024 * 1024)? {
                        Some(bytes) => crate::game_archive::decode_text(&bytes).map(Some),
                        None if path == entry_path => {
                            Err(format!("{}:{path}: 列出的语言文件已消失", source.label()))
                        }
                        None => {
                            for previous in sources[..index].iter().rev() {
                                if let Some(bytes) = previous.read(path, 16 * 1024 * 1024)? {
                                    return crate::game_archive::decode_text(&bytes).map(Some);
                                }
                            }
                            archive_read(path)
                        }
                    }
                };
                read_into(&path, &local_read, &mut languages, &mut warnings);
            }
        }
    }

    (languages, warnings)
}

/// Resolve only complete names. An incomplete translation must not masquerade as a game label.
pub fn resolve(raw: &str, dictionary: &Dictionary) -> Option<String> {
    static TOKEN: OnceLock<regex::Regex> = OnceLock::new();
    let token = TOKEN.get_or_init(|| regex::Regex::new(r"@@([^@]+)@@").unwrap());
    let mut value = raw.trim().to_owned();
    if value.is_empty() {
        return None;
    }
    for _ in 0..8 {
        if !value.contains("@@") {
            return Some(value);
        }
        let mut complete = true;
        let next = token
            .replace_all(&value, |capture: &regex::Captures<'_>| {
                dictionary.get(&capture[1]).cloned().unwrap_or_else(|| {
                    complete = false;
                    capture[0].to_owned()
                })
            })
            .into_owned();
        if !complete || next == value {
            return None;
        }
        value = next;
    }
    None
}

pub fn names(raw: &str, languages: &Languages) -> Dictionary {
    languages
        .iter()
        .filter_map(|(language, dictionary)| {
            resolve(raw, dictionary).map(|name| (language.clone(), name))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_preserve_game_text_and_resolve_nested_tokens() {
        let text = "SiiNunit {\nlocalization_db : .localization {\n key: 2\n key[0]: \"brand\"\n val: 2\n val[0]: \"\\xe6\\xb5\\x8b\\xe8\\xaf\\x95\"\n key[1]: \"name\"\n val[1]: \"@@brand@@ \\\"MX-13\\\"\"\n}\n}\n";
        let dictionary = parse(text).unwrap();
        assert_eq!(resolve("@@name@@", &dictionary).unwrap(), "测试 \"MX-13\"");
        assert_eq!(resolve("Highline", &dictionary).unwrap(), "Highline");
        assert!(resolve("Name @@missing@@", &dictionary).is_none());
        let cycle = BTreeMap::from([("a".into(), "@@b@@".into()), ("b".into(), "@@a@@".into())]);
        assert!(resolve("@@a@@", &cycle).is_none());
    }
    #[test]
    fn mismatched_localization_arrays_are_rejected() {
        assert!(parse(
            "SiiNunit {\nlocalization_db : .l {\n key: 1\n key[0]: \"x\"\n val: 0\n}\n}\n"
        )
        .is_err());
    }
    #[test]
    fn implicit_arrays_ignore_commented_out_game_keys() {
        let text = "SiiNunit {\nlocalization_db : .l {\n # key[]: \"old\"\n key[]: \"part\"\n # val[]: \"old\"\n val[]: \"真实名称\"\n}\n}\n";
        assert_eq!(
            parse(text).unwrap(),
            BTreeMap::from([("part".into(), "真实名称".into())])
        );
    }
    #[test]
    fn comments_and_multiline_strings_do_not_request_missing_includes() {
        let text = "/*\n@include \"missing.sui\"\n*/\nSiiNunit {\nlocalization_db : .l {\nkey[]: \"example\"\nval[]: \"first\n@include \\\"fake.sui\\\"\nlast\"\n}\n}";
        let read = |path: &str| {
            assert_eq!(path, "locale/en_gb/local.sii");
            Ok(Some(text.into()))
        };
        let expanded = expand("locale/en_gb/local.sii", &read, &mut Vec::new(), &mut 0)
            .unwrap()
            .unwrap();
        assert_eq!(expanded, text);
        assert!(parse(&expanded).unwrap()["example"].contains("fake.sui"));
    }

    #[test]
    fn includes_expand_and_reject_cycles_or_escape() {
        let files = BTreeMap::from([
            (
                "locale/en_gb/local.sii",
                "SiiNunit {\nlocalization_db : .l {\n@include \"names.sui\"\n}\n}",
            ),
            (
                "locale/en_gb/names.sui",
                "key[]: \"name\"\nval[]: \"Game name\"\n",
            ),
        ]);
        let read = |path: &str| Ok(files.get(path).map(|text| text.to_string()));
        let text = expand("locale/en_gb/local.sii", &read, &mut Vec::new(), &mut 0)
            .unwrap()
            .unwrap();
        assert_eq!(parse(&text).unwrap()["name"], "Game name");
        assert!(include_path("locale/en_gb/local.sii", "../../secret.txt").is_err());
        let cycle = |_: &str| Ok(Some("@include \"local.sii\"".into()));
        assert!(expand("locale/en_gb/local.sii", &cycle, &mut Vec::new(), &mut 0).is_err());
    }
    #[test]
    #[ignore = "Requires an installed game; reads only locale.scs"]
    fn installed_game_dictionaries() {
        let game = std::env::var("ETS2_GAME_DIR").expect("Set ETS2_GAME_DIR");
        let (languages, warnings) = load(Path::new(&game), &[]);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert!(languages["en"].len() > 10_000);
        assert!(languages["zh_cn"].len() > 10_000);
        assert_eq!(languages["zh_cn"]["transmission"], "变速器");
        assert_eq!(
            resolve("@@engine_mx_13_315@@", &languages["en"]).unwrap(),
            "MX-13 315"
        );
        eprintln!(
            "English keys: {}; Chinese keys: {}",
            languages["en"].len(),
            languages["zh_cn"].len()
        );
    }
}

#[cfg(test)]
mod driver_name_tests {
    use super::*;

    #[test]
    fn names_use_explicit_decimal_indices_not_line_order() {
        let names = parse_driver_names("\u{feff}SiiNunit {\ndriver_names : .names {\nname[12]: \"+Synthetic B\"\nname[00]: \"Synthetic \\\"A\\\"\"\n}\n}").unwrap();
        assert_eq!(names["driver.12"], "Synthetic B");
        assert_eq!(names["driver.0"], "Synthetic \"A\"");
        assert_eq!(names.len(), 2);
        for fields in [
            "name[0]: \"A\"\nname[00]: \"B\"",
            "name[]: \"A\"",
            "name[-1]: \"A\"",
            "name[1]: \"+\"",
        ] {
            assert!(parse_driver_names(&format!(
                "SiiNunit {{\ndriver_names : .names {{\n{fields}\n}}\n}}"
            ))
            .is_err());
        }
    }

    #[test]
    fn game_name_loading_handles_includes_missing_and_malformed_tables() {
        let temp = tempfile::tempdir().unwrap();
        let game = temp.path().join("game");
        let root = temp.path().join("definitions");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::create_dir_all(root.join("locale/en_gb")).unwrap();
        let sources = crate::catalog::disk_sources(&[(root.clone(), "synthetic".into())]);
        let (names, warnings, complete) = load_driver_names(&game, &sources);
        assert!(complete && names.is_empty() && warnings.is_empty());
        std::fs::write(
            root.join("locale/en_gb/driver_names.sii"),
            "SiiNunit {\ndriver_names : .names {\n@include \"drivers.sui\"\n}\n}",
        )
        .unwrap();
        std::fs::write(
            root.join("locale/en_gb/drivers.sui"),
            "name[003]: \"+Synthetic Driver\"\n",
        )
        .unwrap();
        let (names, warnings, complete) = load_driver_names(&game, &sources);
        assert!(complete && warnings.is_empty());
        assert_eq!(names["driver.3"], "Synthetic Driver");
        std::fs::write(
            root.join("locale/en_gb/drivers.sui"),
            "name[3]: \"A\"\nname[03]: \"B\"\n",
        )
        .unwrap();
        let (names, warnings, complete) = load_driver_names(&game, &sources);
        assert!(!complete && names.is_empty() && warnings.len() == 1);
    }

    #[test]
    #[ignore = "read-only installed game check; requires ETS2_DRIVER_GAME"]
    fn installed_english_driver_names() {
        let game = PathBuf::from(std::env::var("ETS2_DRIVER_GAME").unwrap());
        let (names, warnings, complete) = load_driver_names(&game, &[]);
        assert!(complete, "{warnings:?}");
        assert!(names.len() >= 100);
        assert!(names.contains_key("driver.0"));
        eprintln!("Installed English driver names: {}", names.len());
    }
}

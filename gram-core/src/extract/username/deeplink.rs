use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;
use url::Url;
use wildcard::Wildcard;

const PATTERNS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:https?://)?[^\s/$.?#].\S*").unwrap());

pub fn extract_usernames_json(text: &str) -> HashSet<String> {
    PATTERNS
        .find_iter(text)
        .filter_map(|text| get_username(text.as_str()))
        .collect()
}

pub fn get_username(link: &str) -> Option<String> {
    let ret = tg_me_parse(link).or(tg_parse(link))?.username();
    Some(ret)
}

/// 检查链接是否为deeplink，并提取用户名
/// 参考: https://core.telegram.org/api/links
/// TODO: 完整实现
#[derive(Debug, Eq, PartialEq)]
enum DeepLink {
    TMe {
        schema: &'static str,
        username: String,
    },
    TG {
        username: String,
    },
}
impl DeepLink {
    fn username(&self) -> String {
        match self {
            DeepLink::TMe { username, .. } => username.clone(),
            DeepLink::TG { username } => username.clone(),
        }
    }
}

fn tg_me_parse(url: &str) -> Option<DeepLink> {
    let mut patterns = vec![];
    for domain in ["t.me", "telegram.me", "telegram.dog"] {
        patterns.push(("https", format!("https://{domain}/*")));
        patterns.push(("http", format!("http://{domain}/*")));
        patterns.push(("", format!("{domain}/*")));
    }
    patterns
        .iter()
        .map(|(schema, pat)| (schema, Wildcard::new(pat.as_bytes()).unwrap()))
        .filter_map(|(schema, pat)| {
            let a = pat.captures(url.as_bytes())?;
            let uri = str::from_utf8(a[0]).ok()?;
            Some((schema, uri))
        })
        .filter_map(|(schema, uri)| {
            let username = get_uri_username(uri)?.to_string();
            Some(DeepLink::TMe { schema, username })
        })
        .next()
}

fn tg_parse(url: &str) -> Option<DeepLink> {
    let url_no_schema = if url.starts_with("tg://") {
        &url[5..]
    } else if url.starts_with("tg:") {
        &url[3..]
    } else {
        return None;
    };
    let url = Url::parse(&format!("http://t.me/{url_no_schema}")).ok()?;
    let username = url
        .path()
        .eq("/resolve")
        .then(|| {
            url.query_pairs()
                .filter_map(|(k, v)| k.eq("domain").then_some(v))
                .next()
        })
        .flatten()
        .filter(|x| is_username(&x.to_string()))?
        .to_string();
    Some(DeepLink::TG { username })
}

fn get_uri_username(uri: &str) -> Option<&str> {
    uri.split('?')
        .next()?
        .split('/')
        .next()
        .filter(|&x| is_username(x))
}

fn is_username(text: &str) -> bool {
    (![
        "www",
        "addemoji",
        "addlist",
        "addstickers",
        "addtheme",
        "auth",
        "boost",
        "confirmphone",
        "contact",
        "giftcode",
        "invoice",
        "joinchat",
        "login",
        "m",
        "nft",
        "proxy",
        "setlanguage",
        "share",
        "socks",
        "web",
        "a",
        "k",
        "z",
    ]
    .contains(&text))
        && text.len() != 1
        && !text.contains(' ')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// t.me/path?query
    /// http://t.me/path?query
    /// https://t.me/path?query
    fn parse_tg_me() {
        let url = tg_me_parse("t.me/path?query").unwrap();
        assert_eq!(
            url,
            DeepLink::TMe {
                schema: "",
                username: "path".to_owned()
            }
        );
        let url = tg_me_parse("http://t.me/path?query").unwrap();
        assert_eq!(
            url,
            DeepLink::TMe {
                schema: "http",
                username: "path".to_owned()
            }
        );
        let url = tg_me_parse("https://t.me/my-user?query").unwrap();
        assert_eq!(
            url,
            DeepLink::TMe {
                schema: "https",
                username: "my-user".to_owned()
            }
        );
        assert!(tg_me_parse("https://thisisadomain.com").is_none());
        assert!(tg_me_parse("https://thisisadomain.tg.me").is_none());
    }

    #[test]
    fn test_t_me() {
        assert_eq!(
            get_username("t.me/my-username?query"),
            Some("my-username".to_owned())
        );
        assert_eq!(
            get_username("telegram.me/my-username?query"),
            Some("my-username".to_owned())
        );
        assert_eq!(
            get_username("telegram.dog/my-username?query"),
            Some("my-username".to_owned())
        );
    }
    #[test]
    fn test_http_t_me() {
        assert_eq!(
            get_username("http://t.me/my-username?query"),
            Some("my-username".to_owned())
        );
    }
    #[test]
    fn test_https_t_me() {
        assert_eq!(
            get_username("https://t.me/my-username?query"),
            Some("my-username".to_owned())
        );
    }
    #[test]
    fn test_tg() {
        assert_eq!(
            get_username("tg://resolve?domain=my-username&other=query"),
            Some("my-username".to_owned())
        );
    }
    #[test]
    fn test_anti() {
        // not username
        assert_eq!(get_username("http://t.me/v"), None);
        assert_eq!(get_username("tg://resolve?domain=v"), None);
        assert_eq!(get_username("https://t.me/not a url"), None);
        // malformed url
        assert_eq!(get_username("ftp://t.me/v"), None);
        assert_eq!(get_username("tg://resolve?domains=v"), None);
    }

    #[test]
    fn test_batch() {
        let text = "\
                http://t.me/my-username1 \
                https://t.me/my-username2 \
                t.me/my-username3 \
                tg://resolve?domain=my-username4 \
                tg:resolve?domain=my-username5 \
                ";
        assert_eq!(
            extract_usernames_json(text),
            HashSet::from([
                "my-username1".to_owned(),
                "my-username2".to_owned(),
                "my-username3".to_owned(),
                "my-username4".to_owned(),
                "my-username5".to_owned(),
            ])
        );
    }
}

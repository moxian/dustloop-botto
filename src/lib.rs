mod prelude;

mod parse;
mod visitors;
mod parse_ext_traits;


const WIKI_URL: &str = "https://www.dustloop.com/wiki/api.php";

#[derive(serde::Deserialize)]
struct Cred {
    name: String,
    password: String,
}

async fn get_existing_page_text(api: &mediawiki::api::Api, page: &str) -> Option<String> {
    let params = api.params_into(&[
        ("action", "parse"),
        ("page", page),
        ("prop", "wikitext"),
        ("formatversion", "2"), //cargo cult
    ]);
    let res = api.post_query_api_json(&params).await.unwrap();
    let err = &res.as_object().unwrap().get("error");
    if let Some(err) = err {
        let code = err.as_object().unwrap()["code"].as_str().unwrap();
        if code == "missingtitle" {
            return None;
        }
        panic!("{:?}", err);
    }
    let text = res.as_object().unwrap()["parse"].as_object().unwrap()["wikitext"]
        .as_str()
        .unwrap()
        .to_string();
    Some(text)
}


fn colorize_page(existing_text: &str) -> String {
    let parsed = parse_wiki_text::Configuration::new(&parse_wiki_text::ConfigurationSource {
        link_trail: "/^([a-z]+)(.*)$/sD",

        category_namespaces: &[],
        extension_tags: &["big", "tabber", "gallery", "nowiki"],
        file_namespaces: &[],
        magic_words: &[],
        protocols: &[],
        redirect_magic_words: &[],
    })
    .parse(existing_text);

    let warnings = parsed
        .warnings
        .into_iter()
        .filter(|w| {
            ![
                parse_wiki_text::WarningMessage::StrayTextInTable,
                parse_wiki_text::WarningMessage::RepeatedEmptyLine,
                parse_wiki_text::WarningMessage::InvalidLinkSyntax, // huh??
            ]
            .contains(&w.message)
        })
        .collect::<Vec<_>>();
    for w in &warnings {
        println!(
            "{}: {:?}",
            w.message,
            &existing_text[(w.start.max(10) - 10)..(w.end + 10).min(existing_text.len())]
        );
    }
    assert!(warnings.is_empty());

    // let mut visitor = ColorVisitor::default();
    use parse::WikiVisitor;
    let mut visitor = visitors::ComboTableVisitor::default();
    visitor.set_base_text(existing_text);
    parse::visit_nodes(&mut visitor, &parsed.nodes, existing_text);

    let replacements = visitor.replacements;
    {
        let mut last = 0;
        for r in &replacements {
            assert!(last < r.1.start);
            last = r.1.end;
        }
    }
    let mut out = String::new();
    let mut last = 0;
    for (rep, rang) in &replacements {
        out += &existing_text[last..rang.start];
        out += rep;
        last = rang.end;
    }
    out += &existing_text[last..];

    out
}

pub async fn stuff() {
    let cred: Cred =
        serde_json::from_str(&std::fs::read_to_string("bot-creds.json").unwrap()).unwrap();
    let mut api = mediawiki::api::Api::new(WIKI_URL).await.unwrap();
    api.set_user_agent("dustloop botto (by moxian)");
    api.set_edit_delay(Some(100));
    api.login(cred.name, cred.password).await.unwrap();
    // let token = api.get_edit_token().await.unwrap();

    let existing_text = get_existing_page_text(&api, "GGST/Ramlethal Valentine/Combos")
        .await
        .unwrap();
    // let existing_text = "===<big>{{clr|1|5P}}</big>===".to_string();
    // println!("{}", existing_text);
    let new_text = colorize_page(&existing_text);
    println!("{}", new_text)
}


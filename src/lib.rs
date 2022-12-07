mod prelude;

mod api;
mod parse;
mod parse_ext_traits;
mod visitors;

use anyhow::Context;

const WIKI_URL: &str = "https://www.dustloop.com/wiki/api.php";

#[derive(serde::Deserialize)]
struct Cred {
    name: String,
    password: String,
}

fn colorize_page(title: &str, existing_text: &str) -> anyhow::Result<String> {
    let config: visitors::ColorConfig;
    let config_file = match title {
        t if t.starts_with("GGST/") => "data/color/ggst.json5",
        t if t.starts_with("GGACR/") => "data/color/ggacr.json5",
        _ => panic!("{:?}", title),
    };

    config = json5::from_str(&std::fs::read_to_string(config_file).unwrap()).unwrap();
    let mut visitor = visitors::ColorVisitor::new(config);
    let skip_errors = ["GGACR/Venom/Combos"].contains(&title);
    let out = parse::transform_text(existing_text, &mut visitor, skip_errors);
    out
}

fn templatize_combo(existing_text: &str) -> anyhow::Result<String> {
    let mut visitor = visitors::ComboTableVisitor::new();
    let out = parse::transform_text(existing_text, &mut visitor, false);
    out
}

fn find_n_replace(existing_text: &str, config: &FindReplaceConfig) -> anyhow::Result<String> {
    let mut visitor = visitors::FindReplaceVisitor::new(config);
    let out = parse::transform_text(existing_text, &mut visitor, false);
    out
}

fn dump_file(cat: &str, file: &str, content: &str) {
    let p = std::path::Path::new("out")
        .join(cat)
        .join(file.replace("/", "_"));
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    let mut f = std::fs::File::create(p).unwrap();
    f.write_all(content.as_bytes()).unwrap();
}

use std::io::Write;

use clap::Parser;
use visitors::FindReplaceConfig;
#[derive(clap::Parser, Debug)]
struct Args {
    #[clap(long)]
    mode: String,

    #[clap(long)]
    apply: bool,

    #[clap(long)]
    config: Option<String>,
    #[clap(long)]
    page: Option<String>,
}

#[derive(serde::Deserialize)]
struct SkipConfig {
    skip_pages: Vec<String>,
}

pub async fn stuff() {
    let args = Args::parse();
    let cred: Cred = json5::from_str(&std::fs::read_to_string("bot-creds.json5").unwrap()).unwrap();
    let mut api = mediawiki::api::Api::new(WIKI_URL).await.unwrap();
    api.set_user_agent("dustloop botto (by moxian)");
    api.set_edit_delay(Some(100));
    api.login(cred.name, cred.password).await.unwrap();
    let token = &api.get_edit_token().await.unwrap();

    // all_pages = vec!["User:Moxian/Sandbox".into()];

    match args.mode.as_str() {
        "color" => {
            let mut all_pages = api::all_pages_with_prefix(&api, "GGACR/").await;
            let skip_config: SkipConfig =
                json5::from_str(&std::fs::read_to_string("data/skip_pages.json5").unwrap())
                    .unwrap();
            let skip_pages = skip_config.skip_pages.as_slice();

            all_pages = all_pages
                .into_iter()
                // .filter(|p| p.as_str() >= "GGACR/Offense")
                .filter(|p| !skip_pages.contains(&p))
                .collect();

            for title in all_pages.iter() {
                println!("{}", title);
                let (page_meta, content) = api::get_existing_page_text(&api, title).await.unwrap();
                let existing_text = content.as_str();
                let new_text = colorize_page(title, existing_text);
                if true {
                    continue;
                }
                let new_text = new_text.unwrap();
                // println!("{}", new_text)
                // return
                if args.apply && false {
                    println!("Editing..  {} ", title);
                    api::edit_page(
                        &api,
                        token,
                        &page_meta,
                        &new_text,
                        "Switch clr usage from numbers to letters",
                        false,
                    )
                    .await
                    .unwrap();
                    // return;
                }
            }
        }
        "combo" => {
            let page = "GGST/Jack-O/Combos";
            let (_page_meta, content) = api::get_existing_page_text(&api, page).await.unwrap();
            let new_text = templatize_combo(&content).unwrap();
            dump_file("combo", page, &new_text);
            // println!("{}", new_text);
        }
        "movecard" => {
            let page = args.page.as_deref().unwrap();
            let (_page_meta, content) = api::get_existing_page_text(&api, page).await.unwrap();
            let mut visitor = visitors::movecard::MoveCardVisitor::new();
            let new_text = parse::transform_text(&content, &mut visitor, false).unwrap();
            dump_file(&args.mode, page, &new_text);
        }
        "findnreplace" => {
            #[derive(serde::Deserialize)]
            #[serde(untagged)]
            enum PagesSpec {
                List(Vec<String>),
                Spec { prefix: String, pattern: String },
            }
            #[derive(serde::Deserialize)]
            struct Config {
                pages: PagesSpec,
                changes: visitors::FindReplaceConfig,
                #[serde(default)]
                apply: bool,
                #[serde(default)]
                comment: String,
                isminor: Option<bool>,
            }
            let config: Config = json5::from_str(
                &std::fs::read_to_string(args.config.as_ref().context("specify --config").unwrap())
                    .unwrap(),
            )
            .map_err(|e| {
                println!("{}", e);
            })
            .unwrap();
            let pages = match config.pages {
                PagesSpec::List(p) => p,
                PagesSpec::Spec { prefix, pattern } => {
                    let re = regex::Regex::new(&pattern).unwrap();
                    api::all_pages_with_prefix(&api, &prefix)
                        .await
                        .into_iter()
                        .filter(|p| re.is_match(p))
                        .collect::<Vec<_>>()
                }
            };
            println!("pages list: {:?}", pages);
            for page in &pages {
                let (page_meta, content) = api::get_existing_page_text(&api, page).await.unwrap();
                println!("Page: {}", page);
                let new_text = find_n_replace(&content, &config.changes).unwrap();
                let file = std::path::Path::new("out/find_n_repalce").join(page.replace("/", "_"));
                std::fs::create_dir_all(file.parent().unwrap()).unwrap();
                let mut f = std::fs::File::create(file).unwrap();
                f.write_all(new_text.as_bytes()).unwrap();

                if new_text != content {
                    println!(".. done");
                } else {
                    println!(".. no changes!");
                    continue;
                }

                if args.apply {
                    if !config.apply {
                        panic!("noaply in config")
                    }
                    if config.comment.is_empty() {
                        panic!("no comment in config")
                    }
                    let isminor = config.isminor.unwrap();
                    api::edit_page(&api, token, &page_meta, &new_text, &config.comment, isminor)
                        .await
                        .unwrap();
                }
            }
            // println!("{}", new_text);
        }
        "stuff" => {
            let mut all_pages = api::all_pages_with_prefix(&api, "GGST").await;
            all_pages = all_pages
                .into_iter()
                // .filter(|p| p.as_str() >= "GGACR/Offense")
                // .filter(|p| !skip_pages.contains(&p))
                .collect();
            let url_re = regex::Regex::new(r"https?://(www\.)?dustloop.com/\S*").unwrap();
            for title in &all_pages {
                println!("{}", title);
                let (_, content) = api::get_existing_page_text(&api, title).await.unwrap();
                let matches = url_re
                    .find_iter(&content)
                    .map(|m| m.as_str())
                    .collect::<Vec<_>>();
                if !matches.is_empty() {
                    println!("found {:?} matches: {:?}", matches.len(), matches);
                }
            }
        }
        _ => panic!(),
    }
}

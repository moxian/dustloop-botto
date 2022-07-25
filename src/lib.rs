mod prelude;

mod api;
mod parse;
mod parse_ext_traits;
mod visitors;

const WIKI_URL: &str = "https://www.dustloop.com/wiki/api.php";

#[derive(serde::Deserialize)]
struct Cred {
    name: String,
    password: String,
}

fn colorize_page(title: &str, existing_text: &str) -> anyhow::Result<String> {
    assert!(title.starts_with("GGST/"));
    let config: visitors::ColorConfig =
        json5::from_str(&std::fs::read_to_string("data/color/ggst.json5").unwrap()).unwrap();
    let mut visitor = visitors::ColorVisitor::new(config);
    let out = parse::transform_text(existing_text, &mut visitor);
    out
}

fn templatize_combo(existing_text: &str) -> anyhow::Result<String> {
    let mut visitor = visitors::ComboTableVisitor::new();
    let out = parse::transform_text(existing_text, &mut visitor);
    out
}

use clap::Parser;
#[derive(clap::Parser, Debug)]
struct Args {
    #[clap(long)]
    mode: String,

    #[clap(long)]
    apply: bool,
}

pub async fn stuff() {
    let args = Args::parse();
    let cred: Cred = json5::from_str(&std::fs::read_to_string("bot-creds.json5").unwrap()).unwrap();
    let mut api = mediawiki::api::Api::new(WIKI_URL).await.unwrap();
    api.set_user_agent("dustloop botto (by moxian)");
    api.set_edit_delay(Some(100));
    api.login(cred.name, cred.password).await.unwrap();
    let token = &api.get_edit_token().await.unwrap();

    let mut all_pages = api::all_pages_with_prefix(&api, "GGST/").await;
    let skipped: &[&str] = &[];
    all_pages = all_pages
        .into_iter()
        // .filter(|p| p.as_str() >= "GGST/Ino")
        .filter(|p| !skipped.contains(&p.as_str()))
        .collect();

    // all_pages = vec!["User:Moxian/Sandbox".into()];

    for title in &all_pages {
        println!("{}", title);
        let (existing_text, oid) = api::get_existing_page_text(&api, title).await.unwrap();
        let existing_text = existing_text.as_str();
        let new_text;
        match args.mode.as_str() {
            "color" => new_text = colorize_page(title, existing_text),
            "combo" => new_text = templatize_combo(existing_text),
            _ => panic!("unknown mode {:?}", args.mode),
        };
        let new_text = new_text.unwrap();
        // println!("{}", new_text)
        // return
        if args.apply {
            println!("Editing..  {} ", title);
            api::edit_page(
                &api,
                token,
                title,
                "Switch clr usage from numbers to letters",
                true,
                &new_text,
                oid,
            )
            .await
            .unwrap();
            return;
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RevId(i64);

pub async fn get_existing_page_text(
    api: &mediawiki::api::Api,
    page: &str,
) -> Option<(String, RevId)> {
    let params = api.params_into(&[
        ("action", "parse"),
        ("page", page),
        ("prop", "revid|wikitext"),
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
    let text = res["parse"]["wikitext"].as_str().unwrap().to_string();
    let revid = RevId(res["parse"]["revid"].as_i64().unwrap());
    Some((text, revid))
}

pub async fn all_pages_with_prefix(api: &mediawiki::api::Api, prefix: &str) -> Vec<String> {
    let mut params = api.params_into(&[
        ("action", "query"),
        ("list", "allpages"),
        ("apprefix", prefix),
    ]);
    let mut pages = vec![];
    loop {
        let res = api.post_query_api_json(&params).await.unwrap();
        let err = &res.as_object().unwrap().get("error");
        if let Some(err) = err {
            panic!("{:?}", err);
        }
        pages.extend(
            res["query"]["allpages"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| p["title"].as_str().unwrap().to_string()),
        );
        let cont = res.as_object().unwrap().get("continue");
        if let Some(cont) = cont {
            let apcont = cont["apcontinue"].as_str().unwrap();
            params.extend(api.params_into(&[("apcontinue", apcont)]));
        } else {
            break;
        }
    }

    pages
}

pub async fn edit_page(
    api: &mediawiki::api::Api,
    token: &str,
    title: &str,
    summary: &str,
    is_minor: bool,
    content: &str,
    revid: RevId,
) -> anyhow::Result<()> {
    let params = api.params_into(&[
        ("action", "edit"),
        ("title", title),
        ("text", content),
        ("summary", summary),
        ("minor", &format!("{}", is_minor)),
        ("baserevid", &format!("{}", revid.0)),
        ("token", token), // must be last
    ]);
    let res = api.post_query_api_json(&params).await.unwrap();
    println!("result: {:?}", res);

    let err = &res.as_object().unwrap().get("error");
    if let Some(err) = err {
        // let code = err.as_object().unwrap()["code"].as_str().unwrap();
        println!("error {:?}", err);
        anyhow::bail!("failed to edit: {:?}", err);
    }
    Ok(())
}

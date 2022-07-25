pub async fn get_existing_page_text(api: &mediawiki::api::Api, page: &str) -> Option<String> {
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

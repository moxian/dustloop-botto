#[derive(Debug, Clone, Copy)]
pub struct PageId(i64);
#[derive(Debug, Clone, Copy)]
pub struct RevId(i64);

pub struct PageMeta {
    pub title: String,
    pub revid: RevId,
    pub pageid: PageId,
}

pub async fn get_existing_page_text(
    api: &mediawiki::api::Api,
    page: &str,
) -> Option<(PageMeta, String)> {
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
    let pageid = PageId(res["parse"]["pageid"].as_i64().unwrap());
    let title = res["parse"]["title"].as_str().unwrap().to_string();
    Some((
        PageMeta {
            title,
            revid,
            pageid,
        },
        text,
    ))
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
    page: &PageMeta,
    content: &str,
    summary: &str,
    is_minor: bool,
) -> anyhow::Result<()> {
    let mut params = api.params_into(&[
        ("action", "edit"),
        ("title", page.title.as_str()),
        ("text", content),
        ("summary", summary),
        ("baserevid", &format!("{}", page.revid.0)),
        ("token", token), // must be last
    ]);
    if is_minor {
        params.extend(api.params_into(&[("minor", &format!("{}", is_minor))]));
    }

    let res = api.post_query_api_json(&params).await.unwrap();
    // println!("result: {:?}", res);

    // Object({"edit": Object({"contentmodel": String("wikitext"), "nochange": String(""), "pageid": Number(25544), "result": String("Success"), "title": String("GGST/Anji")})})
    // Object({"edit": Object({"contentmodel": String("wikitext"), "newrevid": Number(304981), "newtimestamp": String("2022-07-25T16:53:31Z"), "oldrevid": Number(303643), "pageid": Number(23251), "result": String("Success"), "title": String("GGST/Anji Mito")})})

    let err = &res.as_object().unwrap().get("error");
    if let Some(err) = err {
        // let code = err.as_object().unwrap()["code"].as_str().unwrap();
        println!("error {:?}", err);
        anyhow::bail!("failed to edit: {:?}", err);
    }
    Ok(())
}

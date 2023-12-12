extern crate iron;
#[macro_use]
extern crate mime;

use iron::prelude::*;
use iron::status;

fn main() {
    println!("Server running at http://localhost:3000/");
    Iron::new(get_form).http("localhost:3000").unwrap();
}

fn get_form(_request: &mut Request) -> IronResult<Response> {
    let mut response = Response::new();
    response.set_mut(status::Ok);
    response.set_mut(mime!(Text/Html; Charset=Utf8));
    response.set_mut(
        r#"
        <title>GCD Calculator</title>
        <form action="/gcd" method="post">
            <input type="text" name="n"/>
            <input type="text" name="n"/>
            <button type="submit">Compute GCD</button>
        </form>
    "#,
    );
    Ok(response)
}

use futures::future;
use futures::stream::FuturesUnordered;
use futures::stream::StreamExt;
#[feature(async_closure)]
use reqwest::Client;

const LINKS: [&str; 4] = [
    "example.net/a",
    "example.net/b",
    "example.net/c",
    "example.net/d",
];

async fn fetch_links(client: &Client, link: &str) -> Result<String, reqwest::Error> {
    let res = client.get(link).send().await?;
    Ok(res.text().await?)
}

async fn process_links(client: &Client) {
    let mut futures = FuturesUnordered::new();

    for link in LINKS.iter() {
        let future = fetch_links(client, link);
        futures.push(future);
    }

    while let Some(result) = futures.next().await {
        match result {
            Ok(item) => {
                if item.contains("abc") {
                    let future = fetch_links(client, &item);
                    futures.push(future);
                }
            }
            Err(err) => {
                // Handle error
            }
        }
    }
}

fn stress() {
    let client = Client::new();
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        process_links(&client).await;
    });
    let links = vec![
        // A vec of strings representing links
        "example.net/a".to_owned(),
        "example.net/b".to_owned(),
        "example.net/c".to_owned(),
        "example.net/d".to_owned(),
    ];

    let ref_client = &client; // Need this to prevent client from being moved into the first map
    futures::stream::iter(links)
        .map(async move |link: String| {
            let res = ref_client.get(&link).send().await;

            // res.map(|res| res.text().await.unwrap().to_vec())
            match res {
                // This is where I would usually use `map`, but not sure how to await for a future inside a result
                Ok(res) => Ok(res.text().await.unwrap()),
                Err(err) => Err(err),
            }
        })
        .buffer_unordered(10) // Number of connection at the same time
        .filter_map(|c| future::ready(c.ok())) // Throw errors out, do your own error handling here
        .filter_map(|item| {
            if item.contains("abc") {
                future::ready(Some(item))
            } else {
                future::ready(None)
            }
        })
        .map(async move |sec_link| {
            let res = ref_client.get(&sec_link).send().await;
            match res {
                Ok(res) => Ok(res.text().await.unwrap()),
                Err(err) => Err(err),
            }
        })
        .buffer_unordered(10) // Number of connections for the secondary requests (so max 20 connections concurrently)
        .filter_map(|c| future::ready(c.ok()))
        .for_each(|item| {
            println!("File received: {}", item);
            future::ready(())
        })
        .await;
}

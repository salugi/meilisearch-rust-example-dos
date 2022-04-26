use futures::executor::block_on;
use lazy_static::lazy_static;
use meilisearch_sdk::{client::*};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::File;
use std::io::{stdin, Read};

// instantiate the client. load it once
lazy_static! {
    static ref CLIENT: Client = Client::new("http://localhost:7700", "masterKey");
}

fn main() {
    block_on(async move {
        // build the index
        build_index().await;

        // enter in search queries or quit
        loop {
            println!("Enter a search query or type \"q\" or \"quit\" to quit:");
            let mut input_string = String::new();
            stdin()
                .read_line(&mut input_string)
                .ok()
                .expect("Failed to read line");
            match input_string.trim() {
                "quit" | "q" | "" => {
                    println!("exiting...");
                    break;
                }
                _ => {
                    search(input_string.trim()).await;
                }
            }
        }
        // get rid of the index at the end, doing this only so users don't have the index without knowing
        let _ = CLIENT.delete_index("clothes").await.unwrap();
    })
}

async fn search(query: &str) {
    // make the search query, which excutes and serializes hits into the
    // ClothesDisplay struct
    let query_results = CLIENT
        .index("clothes")
        .search()
        .with_query(query)
        .execute::<ClothesDisplay>()
        .await
        .unwrap()
        .hits;

    // display the query results
    if query_results.len() > 0 {
        for clothes in query_results {
            let display = clothes.result;
            println!("{}", format!("{}", display));
        }
    } else {
        println!("no results...")
    }
}

/*
TODO:
sort by price
add filter?
*/
async fn build_index() {
    // reading and parsing the file
    let mut file = File::open("../assets/clothes.json").unwrap();
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();

    // serialize the string to clothes objects
    let clothes: Vec<Clothes> = serde_json::from_str(&content).unwrap();
    let displayed_attributes = ["article", "cost", "size", "pattern"];

    // Create ranking rules
    // Question: is this the way to do it?
    let ranking_rules = [
        "words",
        "typo",
        "attribute",
        "exactness",
        "cost:asc",
    ];

    let searchable_attributes = ["seaon", "article", "size", "pattern"];

    // create the synonyms hashmap
    let mut synonyms = std::collections::HashMap::new();
    synonyms.insert(
        String::from("sweater"),
        vec![String::from("sweatshirt"), String::from("long-sleeve")],
    );
    synonyms.insert(String::from("sweat pants"), vec![String::from("joggers")]);
    synonyms.insert(
        String::from("t-shirt"),
        vec![String::from("tees"), String::from("tshirt")],
    );

    // set up the synonyms with the client
    let result = CLIENT
        .index("clothes")
        .set_synonyms(&synonyms)
        .await
        .unwrap()
        .wait_for_completion(&CLIENT, None, None)
        .await
        .unwrap();

        if result.is_failure() {
            panic!(
                "Encountered an error while adding synonyms: {:?}",
                result.unwrap_failure()
            );
        }

    // set displayed attributes
    let _ = CLIENT
        .index("clothes")
        .set_displayed_attributes(displayed_attributes)
        .await
        .unwrap();

    // set the ranking rules for the index
    let _ = CLIENT
        .index("clothes")
        .set_ranking_rules(&ranking_rules)
        .await
        .unwrap();

    // set the searchable attributes
    let _ = CLIENT
        .index("clothes")
        .set_searchable_attributes(&searchable_attributes)
        .await
        .unwrap();

    // add the documents
    let result = CLIENT
        .index("clothes")
        .add_or_update(&clothes, Some("id"))
        .await
        .unwrap()
        .wait_for_completion(&CLIENT, None, None)
        .await
        .unwrap();
    if result.is_failure() {
        panic!(
            "Encountered an error while sending the documents: {:?}",
            result.unwrap_failure()
        );
    }
}

/// Base search object.
#[derive(Serialize, Deserialize, Debug)]
pub struct Clothes {
    id: usize,
    seaon: String,
    article: String,
    cost: f32,
    size: String,
    pattern: String,
}

/// Search results get serialized to this struct
#[derive(Serialize, Deserialize, Debug)]
pub struct ClothesDisplay {
    article: String,
    cost: f32,
    size: String,
    pattern: String,
}

impl fmt::Display for ClothesDisplay {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(
            f,
            "\nresult\n article: {},\n price: {},\n size: {},\n pattern: {}\n",
            self.article, self.cost, self.size, self.pattern
        )
    }
}


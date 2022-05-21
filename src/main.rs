use std::future::Future;
use std::mem;

use lazy_static::lazy_static;
use nlprule::Tokenizer;
use rust_bert::pipelines::sentiment::{SentimentModel, SentimentPolarity};
use rust_bert::pipelines::summarization::SummarizationModel;
use serde::{Deserialize, Serialize};
use warp::Filter;

use std::sync::{Arc, Mutex, MutexGuard};
use tokio::task::spawn_blocking;

#[derive(Deserialize, Debug)]
pub struct Query {
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PrickErr {
    Analysis(String),
    InternalAsync(String),
}

impl warp::reject::Reject for PrickErr {}

#[derive(Debug, Serialize, Deserialize)]
struct SentenceAnalysis {
    text: String,
    score: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Analysis {
    sentences: Vec<SentenceAnalysis>,
    total_score: f64,
}

fn analyse_text(
    tokenizer: MutexGuard<Tokenizer>,
    classifier: MutexGuard<SentimentModel>,
    text: &str,
) -> Result<Analysis, Box<dyn std::error::Error>> {
    let input = [
        text,
    ];

    let instr = &input.join(" ");

    let sentences =
        {
            tokenizer
                .pipe(instr)
                .map(|sentence|
                    sentence
                        .text()
                        .trim_start()
                        .trim_end()
                        .to_string()
                        .clone()
                )
                .collect::<Vec<_>>()
        };

    let output =
        {
            classifier
                .predict(
                    &sentences
                        .iter()
                        .map(|item| item.as_str())
                        .collect::<Vec<&str>>(),
                )
        };

    let mut i = 0;
    let mut total_score: f64 = 0.0;

    let mut sentence_analysis = Vec::<SentenceAnalysis>::new();

    for sentiment in output.iter() {
        let sentiment_score =
            match &sentiment.polarity {
                SentimentPolarity::Positive => sentiment.score,
                SentimentPolarity::Negative => -sentiment.score,
            };

        sentence_analysis.push(
            SentenceAnalysis {
                text: sentences[i].to_string().clone(),
                score: sentiment_score,
            }
        );

        total_score += sentiment_score;

        i += 1;
    }

    Ok(
        Analysis {
            sentences: sentence_analysis,
            total_score,
        }
    )
}

async fn handle_analyze(
    tokenizer: Arc<Mutex<Tokenizer>>,
    classifier: Arc<Mutex<SentimentModel>>,
    query: Query,
) -> Result<impl warp::Reply, warp::Rejection> {
    let query = query.text.clone();

    let analysis =
        spawn_blocking(move || {
            let tokenizer = tokenizer.lock().unwrap();
            let classifier = classifier.lock().unwrap();

            analyse_text(
                tokenizer,
                classifier,
                &query,
            )
            .map_err(|err|
                warp::reject::custom(
                    PrickErr::Analysis(
                        format!("{:?}", err),
                    ),
                )
            )
        })
            .await
            .map_err(|err|
                warp::reject::custom(
                    PrickErr::InternalAsync(
                        format!("{:?}", err),
                    ),
                )
            )??;

    Ok(
        warp::reply::json(
            &analysis,
        )
    )
}

#[tokio::main]
async fn main() {
    let tokenizer =
        Arc::new(
            Mutex::new(
                spawn_blocking(
                    || Tokenizer::new(
                        "./en_tokenizer.bin",
                    ).unwrap(),
                ).await.unwrap(),
            ),
        );

    let classifier =
        Arc::new(
            Mutex::new(
                spawn_blocking(
                    || SentimentModel::new(
                        Default::default(),
                    ).unwrap(),
                ).await.unwrap(),
            ),
        );

    let tokenizer_provider =
        warp::any().map(move || tokenizer.clone());

    let sentiment_classifier_provider =
        warp::any().map(move || classifier.clone());

    let analysis =
        warp::post()
            .and(warp::path!("analyze"))
            .and(tokenizer_provider)
            .and(sentiment_classifier_provider)
            .and(
                warp::body::content_length_limit(1024 * 128)
                    .and(warp::body::json::<Query>())
            )
            .and_then(handle_analyze);

    let default =
        warp::path::end()
            .map(|| "its dark and I am lost ▪ chaos".to_string());

    let routes =
        analysis
            .or(
                warp::get()
                    .and(default)
            );

    println!("i cannot see and it is cold");
    println!("if you can read this help");
    println!("its dark and I am lost");
    println!("▪");
    println!("listening on port 7171");

    warp::serve(routes)
        .run(([0, 0, 0, 0], 7171))
        .await;
}
